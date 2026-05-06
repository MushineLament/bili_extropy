use std::{
    borrow::Cow,
    fs::{self, File},
    hash::Hash,
    io::{self, ErrorKind, Seek as _, SeekFrom, Write as _},
    mem,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use crate::{
    api::BiliApi,
    components::{
        download::{MediaInfoAidPayload, MediaInfoBvidPayload},
        downloadtask::{handle::TaskId, load::LoadDownloadtaskTask},
        fetch::handle::Loadable,
        handle::{ECSHandle, ECSHandleResult},
        status::handle::{DownloadruleId, StatusId},
    },
    cookies::{add_cookie_jar, parse_cookies},
    db::Db,
    entity::{
        BvId, MediaAid,
        downloadrule::DownloadruleModel,
        downloadtask,
        media::{self, Media, MediaInfoData, MediaInfoResp, MediaInfoSingle, Page},
        status::StatusModel,
    },
    output::{EntryOuput, IndexAudio, IndexOuput, IndexVideo},
    payload::DashPayload,
    response::{self, Dash, DashData, DashResp},
};
use anyhow::{Result, anyhow};
use api_req::ApiCaller as _;
use bevy::{
    ecs::{change_detection::MaybeLocation, component::Component},
    platform::{
        collections::{HashMap, HashSet, hash_map},
        hash::FixedHasher,
    },
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use bimap::BiHashMap;
use bytes::BytesMut;
use chrono::DateTime;
use futures::{StreamExt, stream::FuturesUnordered};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use migration::OnConflict;
use reqwest::{IntoUrl, Response, header::CONTENT_LENGTH};
use sea_orm::{
    ActiveValue::Unchanged, ColumnTrait, EntityTrait as _, IntoActiveModel as _, QueryFilter,
};
use tempfile::NamedTempFile;
use tracing::{error, info, warn};
use url::Url;

const BAR_TEMPLATE: &str = "{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})";

pub const TEMP_DOWNLOAD_FOLDER: &str = ".temp";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DownloadWay(pub String);

impl DownloadWay {
    pub fn new<T: Into<String>>(str: T) -> Self {
        Self(str.into())
    }
}

impl DownloadPendding for DownloadWay {
    fn to_response(
        &self,
    ) -> impl Future<Output = Result<MediaInfoSingle, DownloadFileError>> + Send {
        let handle = async {
            let anyhow = anyhow::anyhow!("request aid media<{:?}>", self.0);

            let response: std::result::Result<MediaInfoSingle, DownloadFileError> =
                BiliApi::request(MediaInfoBvidPayload {
                    bvid: self.0.clone(),
                })
                .await
                .map_err(|err| DownloadFileError::new(DownloadFileErrorKind::ApiReq(err)));

            if response.is_ok() {
                return response;
            }

            let Ok(aid) = self.0.parse::<MediaAid>() else {
                // if not a mediacid and not a avlid bvid
                error!(
                    "{:?}",
                    anyhow::anyhow!("{:?} error:{:?}", anyhow, MaybeLocation::caller())
                );
                return response;
            };

            let response: Result<MediaInfoSingle, DownloadFileError> =
                BiliApi::request(MediaInfoAidPayload { aid })
                    .await
                    .map_err(|err| DownloadFileError::new(DownloadFileErrorKind::ApiReq(err)));

            response
        };

        handle
    }

    fn media_aid(&self) -> impl Future<Output = Result<MediaAid, DownloadFileError>> {
        async {
            let response = self.to_response().await?;

            let media = response
                .data
                .ok_or(DownloadFileError::new(DownloadFileErrorKind::MediaPage))?;

            Ok(media.aid)
        }
    }

    fn related_task_id(
        &self,
        db: &Db,
    ) -> impl Future<Output = Result<Vec<TaskId>, DownloadFileError>> {
        async {
            let media_id = self.media_aid().await?;

            let realted_taskids = LoadDownloadtaskTask::load_with(db, |select| {
                select.filter(downloadtask::Column::Id.eq(media_id))
            })
            .await
            .map_err(|err| DownloadFileError::new(DownloadFileErrorKind::Db(err)))?;

            Ok(realted_taskids.into_iter().map(|model| model.id).collect())
        }
    }
}

pub trait DownloadPendding {
    fn to_response(
        &self,
    ) -> impl Future<Output = Result<MediaInfoSingle, DownloadFileError>> + Send;

    fn media_aid(&self) -> impl Future<Output = Result<MediaAid, DownloadFileError>>;

    fn related_task_id(
        &self,
        db: &Db,
    ) -> impl Future<Output = Result<Vec<TaskId>, DownloadFileError>>;
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct DownloadHandle(pub ECSHandle<Result<BvId, DownloadFileError>>);

impl DownloadHandle {
    pub fn new<T: Into<String>, R: DownloadPendding + 'static + Send>(
        db: Db,
        bars: MultiProgress,
        cookies: T,
        list: R,
        runtimer: &mut TokioTasksRuntime,
        active_status: Arc<HashMap<i64, StatusModel>>,
        active_downloadrule: Arc<HashMap<i64, DownloadruleModel>>,
        status_related_downloadrule: Arc<
            BiHashMap<StatusId, DownloadruleId, FixedHasher, FixedHasher>,
        >,
    ) -> Self {
        let cookies = cookies.into();
        let task = async move {
            add_cookie_jar(parse_cookies(&cookies));

            let media = list.to_response().await?;

            let MediaInfoSingle {
                code: _,
                data: Some(media),
                message: _,
            } = media
            else {
                return Err(DownloadFileError::new(DownloadFileErrorKind::MediaPage));
            };

            let mediaid = media.aid;

            // 时间符合要求的计算
            let _allow = active_downloadrule.iter().filter(|(_ruleid, model)| {
                DateTime::from_timestamp(media.pubdate as i64, 0)
                    .map(|pubdate| model.default_relation_date(pubdate.naive_utc()))
                    .unwrap_or_else(|| {
                        error!("compute media<{:?}> public date time error,will ignore about time download rule.", mediaid);
                        true
                    })
            });

            let aid = media.aid;
            let bvid = media.bvid.clone();

            let model = crate::entity::media::MediaModel {
                aid: media.aid,
                bv_id: media.bvid.to_owned(),
                title: media.title.to_owned(),
                r#type: media.r#type.to_string(),
                cid: media.cid,
                pic: None,
            };

            let _prikey =
                media::MediaEntity::insert_many([model].into_iter().map(|m| m.into_active_model()))
                    .on_conflict(
                        OnConflict::column(media::Column::Aid)
                            .update_columns([
                                media::Column::BvId,
                                media::Column::Cid,
                                media::Column::Title,
                                media::Column::Type,
                            ])
                            .to_owned(),
                    )
                    .exec_without_returning(&db.db)
                    .await
                    .map_err(|err| {
                        error!(
                            "can't not upsert media<{:?}> in to table, error:{:?}",
                            aid, err
                        );

                        DownloadFileError::new(DownloadFileErrorKind::Db(err))
                    })?;

            let result = download(
                &db,
                &media,
                bars.clone(),
                &Path::new(TEMP_DOWNLOAD_FOLDER),
                active_status.as_ref(),
                active_downloadrule.as_ref(),
                status_related_downloadrule.as_ref(),
            )
            .await
            .map(|_| bvid);

            info!("task finished");
            result
        };

        let handle = runtimer.spawn_background_task(move |_ctx| task);
        Self(ECSHandle::new(handle))
    }

    pub async fn video_response(
        video: Option<&response::Video>,
        tmp_path: &Path,
        pb: DownloadProgressBar,
        total_bytes: Arc<AtomicU64>,
        file_entry_json: &mut EntryOuput,
        index_video: &mut IndexVideo,
    ) -> Option<Result<DownloadFileResponse, DownloadFileError>> {
        let Some(video) = video else {
            return None;
        };

        file_entry_json.update_video(video);
        index_video.update_video(video);

        let tmp = match NamedTempFile::new_in(tmp_path) {
            Ok(tmp) => tmp,
            Err(err) => {
                return Some(Err(err.into()));
            }
        };

        let pendding = match DownloadFilePending::from_tmp_url(
            video.base_url.as_str(),
            TempFilePendding::new("video.m4s", tmp, MediaFileType::Stream(video.id)),
        ) {
            Ok(pending) => pending,
            Err(err) => {
                return Some(Err(err.into()));
            }
        };

        let response = match pendding.with_bg(pb).into_response().await {
            Ok(response) => response.map(|res| {
                if let Ok(size) = res.try_headers_size() {
                    total_bytes.fetch_add(size, std::sync::atomic::Ordering::Relaxed);
                    res.pb.as_ref().map(|pg| pg.inc_length(size));
                }
            }),
            Err(err) => {
                return Some(Err(err.into()));
            }
        };

        Some(Ok(response))
    }

    pub async fn audio(
        audio: Option<&response::Audio>,
        tmp_path: &Path,
        pb: DownloadProgressBar,
        total_bytes: Arc<AtomicU64>,
        file_entry_json: &mut EntryOuput,
        index_video: &mut IndexVideo,
        index_audio: &mut IndexAudio,
    ) -> Option<Result<DownloadFileResponse, DownloadFileError>> {
        let Some(audio) = audio else {
            return None;
        };

        file_entry_json.update_audio(audio);
        index_audio.update_audio(audio);
        index_video.update_audio_id(audio.id);

        let tmp = match NamedTempFile::new_in(tmp_path) {
            Ok(tmp) => tmp,
            Err(err) => {
                return Some(Err(err.into()));
            }
        };

        let pendding = match DownloadFilePending::from_tmp_url(
            audio.base_url.as_str(),
            TempFilePendding::new("audio.m4s", tmp, MediaFileType::Stream(index_video.id)),
        ) {
            Ok(pending) => pending,
            Err(err) => {
                return Some(Err(err.into()));
            }
        };

        let response = match pendding.with_bg(pb).into_response().await {
            Ok(response) => response.map(|res| {
                if let Ok(size) = res.try_headers_size() {
                    total_bytes.fetch_add(size, std::sync::atomic::Ordering::Relaxed);
                    res.pb.as_ref().map(|pg| pg.inc_length(size));
                }
            }),
            Err(err) => {
                return Some(Err(err.into()));
            }
        };

        Some(Ok(response))
    }

    pub async fn danmu_xml(
        tmp_path: &Path,
        cid: i64,
    ) -> Result<DownloadFileResponse, DownloadFileError> {
        let tmp_path = tmp_path.to_path_buf();

        let xml_url = format!("https://api.bilibili.com/x/v1/dm/list.so?oid={}", cid);

        let response = DownloadFileResponse::from_url(
            xml_url,
            TempFilePendding::new(
                "danmaku.xml",
                NamedTempFile::new_in(tmp_path)?,
                MediaFileType::Infomation,
            ),
        )
        .await?
        .with_download(|mut res| async {
            let mut bytes = BytesMut::new();
            while let Some(chunk) = res.response.chunk().await? {
                bytes.extend_from_slice(&chunk);
            }

            // 解压 deflate 数据
            let mut decoder = flate2::bufread::DeflateDecoder::new(&bytes[..]);
            let mut decompressed = Vec::new();
            use std::io::Read;

            decoder.read_to_end(&mut decompressed)?;

            res.tmp.write_all(&decompressed)?;

            Ok(res)
        })
        .await;

        response
    }
}

pub async fn download(
    db: &Db,
    media: &Media,
    bars: MultiProgress,
    tmp_path: &Path,
    active_status: &HashMap<i64, StatusModel>,
    _active_downloadrule: &HashMap<i64, DownloadruleModel>,
    _status_related_downloadrule: &BiHashMap<StatusId, DownloadruleId, FixedHasher, FixedHasher>,
) -> Result<(), DownloadFileError> {
    if !Path::new(tmp_path).exists() {
        let _ = fs::create_dir_all(tmp_path)?;
    }

    if !Path::new(tmp_path).is_dir() {
        return Err(DownloadFileError::new(DownloadFileErrorKind::IO(
            io::Error::new(ErrorKind::NotADirectory, "can't store tmp file"),
        )));
    }

    if active_status.is_empty() {
        return Err(DownloadFileError::new(DownloadFileErrorKind::Status(
            anyhow!("Not any status error"),
        )));
    }

    let folders = active_status
        .iter()
        .map(|(_, model)| Path::new(&model.path).join(&model.name))
        .filter(|path| path.is_dir() || (!path.exists() && fs::create_dir_all(path).is_ok()))
        .collect::<Vec<_>>();

    let system_time = std::time::SystemTime::now();
    let duration_file_create = system_time.duration_since(std::time::UNIX_EPOCH)?;

    let mut file_entry_json = EntryOuput::default();
    file_entry_json.time_update_stamp = duration_file_create.as_millis();
    file_entry_json.update_media(media);

    let total_bytes = Arc::new(AtomicU64::new(0));

    let mut index = IndexOuput::default();

    let response_icon = DownloadFilePending::from_tmp_url(
        media.pic.as_str(),
        TempFilePendding::new(
            "cover.jpg",
            NamedTempFile::new_in(tmp_path)?,
            MediaFileType::Infomation,
        ),
    )?
    .into_response()
    .await?;

    let MediaInfoResp {
        code,
        data,
        message,
    } = BiliApi::request(MediaInfoAidPayload { aid: media.aid }).await?;

    let (
        Some(MediaInfoData {
            owner,
            pages,
            staff: _,
            cid: _,
        }),
        0,
    ) = (data, code)
    else {
        media::MediaEntity::update(media::ActiveModel {
            aid: Unchanged(media.aid),
            ..Default::default()
        })
        .exec(&db.db)
        .await?;

        return Err(DownloadFileError::new(DownloadFileErrorKind::Page(
            anyhow!(
                "Info unreachable media<{}-{}-{}>: {}",
                media.aid,
                media.bvid,
                media.title,
                message.unwrap_or_default()
            ),
        )));
    };

    file_entry_json.update_owner(&owner);

    let mut tmptofolder = FilesIntoFolders::new();

    let mut response_files = HashMap::new();

    let file_index_id = AtomicUsize::new(0);

    response_files.insert(file_index_id.fetch_add(1, Ordering::Relaxed), response_icon);

    let mut index_files = vec![];

    let only1p = pages.len() == 1;
    for Page { cid, page, part } in pages {
        file_entry_json.update_page(cid, page);

        let _filename = if only1p {
            format!("{}-{}", media.aid, media.title)
        } else {
            format!("{}-{}({page})-{part}", media.aid, media.title)
        };
        let DashResp {
            data:
                DashData {
                    dash: Dash { video, audio },
                    timelength,
                },
            ..
        } = BiliApi::request(
            DashPayload::new(
                media.aid,
                cid,
                system_time.duration_since(std::time::UNIX_EPOCH)?.as_secs(),
            )
            .await?,
        )
        .await?;

        file_entry_json.total_time_milli = timelength;

        let mut video = video.into_iter();
        let mut audio = audio.into_iter();

        let (v, a) = (video.next(), audio.next());

        if v.is_none() && a.is_none() {
            // TODO! need more detailed error infomation.
            continue;
        }

        let pb = DownloadProgressBar::default();
        pb.progress_bar(&bars, &part);

        let (mut index_video, mut index_audio): (IndexVideo, IndexAudio) =
            (Default::default(), Default::default());

        let video_index = if let Some(file_video) = DownloadHandle::video_response(
            v.as_ref(),
            tmp_path,
            pb.clone(),
            total_bytes.clone(),
            &mut file_entry_json,
            &mut index_video,
        )
        .await
        {
            let id = file_index_id.fetch_add(1, Ordering::Relaxed);
            response_files.insert(id, file_video?);
            Some(id)
        } else {
            None
        };

        let audio_index = if let Some(file_audio) = DownloadHandle::audio(
            a.as_ref(),
            tmp_path,
            pb.clone(),
            total_bytes.clone(),
            &mut file_entry_json,
            &mut index_video,
            &mut index_audio,
        )
        .await
        {
            let id = file_index_id.fetch_add(1, Ordering::Relaxed);
            response_files.insert(id, file_audio?);
            Some(id)
        } else {
            None
        };

        if let Ok(file_danmu) = DownloadFilePending::from_tmp_url(
            format!(
                "https://api.bilibili.com/x/v2/dm/web/seg.so?type=1&oid={}&segment_index=1",
                media.cid
            ),
            TempFilePendding::new(
                "danmaku.pb",
                NamedTempFile::new_in(tmp_path)?,
                MediaFileType::Infomation,
            ),
        )?
        .into_response()
        .await
        {
            response_files.insert(file_index_id.fetch_add(1, Ordering::Relaxed), file_danmu);
        };

        if let Ok(file_danmu_xml) = DownloadHandle::danmu_xml(tmp_path, media.cid).await {
            response_files.insert(
                file_index_id.fetch_add(1, Ordering::Relaxed),
                file_danmu_xml,
            );
        };

        let _file_entry = {
            file_entry_json.downloaded_bytes = pb.position();
            file_entry_json.page_data.from = "vupload".to_string();
            file_entry_json.is_completed = true;

            let duration_file_update = system_time.duration_since(std::time::UNIX_EPOCH)?;
            file_entry_json.time_create_stamp = duration_file_update.as_millis();

            file_entry_json.total_bytes = total_bytes.load(std::sync::atomic::Ordering::Relaxed);
        };

        index_files.push((audio_index, index_audio, video_index, index_video));
    }

    let mut tasks = FuturesUnordered::new();

    for (index, file) in response_files {
        tasks.push(async move { (index, file.download().await) });
    }

    let mut file_penddings = HashMap::new();

    while let Some((index, result)) = tasks.next().await {
        match result {
            Ok(file) => {
                file_penddings.insert(index, file.tmp);
            }
            Err(err) => {
                error!("download file error:{:?}", err);
            }
        }
    }

    let _file_entry = {
        let file_entry = TempFilePendding::new(
            "entry.json",
            NamedTempFile::new_in(tmp_path)?,
            MediaFileType::Infomation,
        );

        let entry_writer = io::BufWriter::new(&file_entry.tmp);
        serde_json::to_writer_pretty(entry_writer, &file_entry_json)?; // 使用 pretty 格式化输出;

        file_penddings.insert(file_index_id.fetch_add(1, Ordering::Relaxed), file_entry);
    };

    for (audio_index, mut index_audio, video_index, mut index_video) in index_files {
        if let Some(audio) = audio_index
            && let Some(file_audio) = file_penddings.get(&audio)
        {
            let (size, md5) = file_audio
                .get_size_md5()
                .unwrap_or((0, "get md5 error".to_string()));

            index_audio.update_md5_size(md5, size);

            index.audio.push(index_audio);
        }

        if let Some(video) = video_index
            && let Some(file_video) = file_penddings.get(&video)
        {
            let (size, md5) = file_video
                .get_size_md5()
                .unwrap_or((0, "get md5 error".to_string()));

            index_video.update_md5_size(md5, size);

            index.video.push(index_video);
        }
    }

    // index.json
    let _file_index = {
        let ids = index.video.iter().map(|v| v.id).collect::<HashSet<_>>();

        for id in ids {
            let file_index = TempFilePendding::new(
                "index.json",
                NamedTempFile::new_in(tmp_path)?,
                MediaFileType::Stream(id),
            );

            let index_writer = io::BufWriter::new(&file_index.tmp);

            serde_json::to_writer_pretty(index_writer, &index)?; // 使用 pretty 格式化输出;

            file_penddings.insert(file_index_id.fetch_add(1, Ordering::Relaxed), file_index);
        }
    };

    let path = folders
        .iter()
        .map(|folder| folder.join(media.aid.to_string()))
        .map(|aid_foler| aid_foler.join(format!("c_{}", media.cid.to_string())))
        .collect::<Vec<_>>();

    for (_index, file) in file_penddings {
        let r#type = file.r#type;
        let name = file.name.clone();

        let reopon = match file.to_reopon() {
            Ok(result) => result,
            Err(err) => {
                error!("tmp<{}> into reopon error:{:?}", name, err);
                continue;
            }
        };

        for path in path.iter() {
            let reopon = match reopon.try_clone() {
                Ok(result) => result,
                Err(err) => {
                    error!("tmp<{}> reopon try clone error:{:?}", reopon.name, err);
                    continue;
                }
            };

            match r#type {
                MediaFileType::Infomation => {
                    tmptofolder.add(path, reopon);
                }
                MediaFileType::Stream(id) => {
                    let path = path.join(id.to_string());
                    tmptofolder.add(path, reopon);
                }
            }
        }
    }

    tmptofolder.build();

    media::MediaEntity::update(media::ActiveModel {
        aid: Unchanged(media.aid),
        ..Default::default()
    })
    .exec(&db.db)
    .await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaFileType {
    /// cover.jpg
    /// danmaku.pb
    /// danmaku.xml
    /// entry.json
    Infomation,

    // audio.m4s
    // index.json
    // video.m4s
    Stream(i64),
}

#[derive(Debug, Deref, DerefMut)]
pub struct TempFilePendding {
    pub name: Cow<'static, str>,
    pub r#type: MediaFileType,

    #[deref]
    pub tmp: NamedTempFile,
}

impl TempFilePendding {
    pub fn new<T: Into<Cow<'static, str>>>(
        name: T,
        tmp: NamedTempFile,
        r#type: MediaFileType,
    ) -> Self {
        Self {
            name: name.into(),
            tmp,
            r#type,
        }
    }

    pub fn get_size_md5(&self) -> Result<(u64, String), io::Error> {
        let file = self.as_file();

        let size = file.metadata()?.len();

        // 2. 打开一个独立的文件句柄（克隆），避免影响原句柄
        let mut file = self.tmp.as_file().try_clone()?;

        file.seek(SeekFrom::Start(0))?;

        let mut v_context = md5::Context::new();
        let mut v_buffer = [0; 8192];

        loop {
            let n = io::Read::read(&mut file, &mut v_buffer)?;
            if n == 0 {
                break;
            }
            v_context.consume(&v_buffer[..n]);
        }

        let md5_video = v_context.finalize();
        let hex_video = hex::encode(md5_video.as_ref());

        Ok((size, hex_video))
    }

    pub fn to_reopon(self) -> Result<TempFileReopen, io::Error> {
        let reopon = self.tmp.reopen()?;
        Ok(TempFileReopen {
            name: self.name,
            file: reopon,
        })
    }
}

#[derive(Debug)]
pub struct TempFileReopen {
    pub name: Cow<'static, str>,
    pub file: File,
}

impl TempFileReopen {
    pub fn new<T: Into<Cow<'static, str>>>(name: T, tmp: File) -> Self {
        Self {
            name: name.into(),
            file: tmp,
        }
    }

    pub fn try_clone(&self) -> Result<Self, io::Error> {
        Ok(Self {
            name: self.name.clone(),
            file: self.file.try_clone()?,
        })
    }

    pub fn get_size_md5(&self) -> Result<(u64, String), io::Error> {
        let mut file = self.file.try_clone()?;

        let size = file.metadata()?.len();

        let mut v_context = md5::Context::new();
        let mut v_buffer = [0; 8192];

        loop {
            let n = io::Read::read(&mut file, &mut v_buffer)?;
            if n == 0 {
                break;
            }
            v_context.consume(&v_buffer[..n]);
        }

        let md5_video = v_context.finalize();
        let hex_video = hex::encode(md5_video.as_ref());

        Ok((size, hex_video))
    }

    pub fn size(&self) -> Result<u64> {
        let meta = self.file.metadata()?;
        Ok(meta.len())
    }
}

impl PartialEq for TempFileReopen {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for TempFileReopen {}
impl Hash for TempFileReopen {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
pub struct FilesIntoFolders(pub HashMap<PathBuf, HashSet<TempFileReopen>>);

impl FilesIntoFolders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add<P: Into<PathBuf>, T: Into<TempFileReopen>>(
        &mut self,
        folder: P,
        file: T,
    ) -> &mut Self {
        match self.entry(folder.into()) {
            hash_map::Entry::Occupied(mut occupied) => {
                let file = file.into();

                let name = file.name.clone();
                if occupied.get_mut().insert(file) {
                    warn!("temp file to name has replace:{:?}", name);
                }
            }
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(HashSet::from_iter([file.into()]));
            }
        }

        self
    }

    pub fn add_path_and_files<
        P: Into<PathBuf>,
        I: IntoIterator<Item = T>,
        T: Into<TempFileReopen>,
    >(
        &mut self,
        folder: P,
        files: I,
    ) -> &mut Self {
        match self.entry(folder.into()) {
            hash_map::Entry::Occupied(mut occupied) => {
                for file in files {
                    let file = file.into();

                    let name = file.name.clone();
                    if occupied.get_mut().insert(file) {
                        warn!("temp file to name has replace:{:?}", name);
                    }
                }
            }
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(HashSet::from_iter(
                    files.into_iter().map(|file| file.into()),
                ));
            }
        }

        self
    }

    /// 后续计划，实现错误收集，以便支持自行处理
    /// 将 NamedTempFile 移动到指定目录下，并重命名指定文件名
    pub fn build(&mut self) {
        for (folder, name) in self.iter_mut() {
            if !folder.exists() {
                if let Err(err) = fs::create_dir_all(&folder) {
                    error!("folder:{:?},err:{:?}", folder, err);
                };
            }

            if !folder.is_dir() {
                error!("path not is a directoy:{:?}", folder);
                continue;
            }

            let tempfiles = mem::replace(name, HashSet::new());

            let mut to_vec = Vec::from_iter(tempfiles);

            for file_name in to_vec.iter_mut() {
                if let Err(e) = file_name.file.seek(io::SeekFrom::Start(0)) {
                    error!("Failed to seek file {:?} to start: {}", file_name.name, e);
                    continue;
                }

                let target_path = folder.join(file_name.name.as_ref());

                if target_path.exists() {
                    error!("target file exists error:{:?}", target_path);
                    continue;
                }

                let mut target_file = match File::create(&target_path) {
                    Ok(file) => file,
                    Err(err) => {
                        error!("target file:{:?} create error:{:?}", target_path, err);
                        continue;
                    }
                };

                match io::copy(&mut file_name.file, &mut target_file) {
                    Ok(_bytes) => {
                        // info!("Copied {} bytes to {:?}", bytes, target_path);
                        ()
                    }
                    Err(err) => error!("copy error: {:?}", err),
                }
            }

            name.extend(to_vec);
        }
    }
}

#[derive(Debug, Deref, DerefMut, Clone)]
pub struct DownloadProgressBar(pub ProgressBar);

impl DownloadProgressBar {
    pub fn new(len: u64) -> Self {
        Self(ProgressBar::new(len))
    }

    pub fn progress_bar(&self, bars: &MultiProgress, part: &str) -> &Self {
        bars.add(self.0.clone());

        self.set_message(format!("{:<10}", part));
        self.set_style(
            ProgressStyle::with_template(BAR_TEMPLATE)
                .unwrap()
                .progress_chars("#>-"),
        );

        self
    }
}

impl Default for DownloadProgressBar {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Debug)]
pub struct DownloadFilePending {
    pub pb: Option<DownloadProgressBar>,
    pub tmp: TempFilePendding,
    pub url: Url,
}

impl DownloadFilePending {
    pub fn new<T: IntoUrl>(url: T, tmp: TempFilePendding) -> Result<Self, DownloadFileError> {
        Ok(Self {
            pb: None,
            url: url.into_url()?,
            tmp,
        })
    }

    #[track_caller]
    pub fn from_tmp_url<T: IntoUrl>(
        url: T,
        tmp: TempFilePendding,
    ) -> Result<Self, DownloadFileError> {
        let url = url.into_url()?;

        Ok(Self { pb: None, tmp, url })
    }

    pub fn new_with_bg<T: IntoUrl>(
        url: T,
        tmp: TempFilePendding,
        pg: DownloadProgressBar,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            pb: Some(pg),
            tmp,
            url: url.into_url()?,
        })
    }

    pub async fn into_response(self) -> Result<DownloadFileResponse, DownloadFileError> {
        let Self { pb, tmp, url } = self;

        let mut response =
            DownloadFileResponse::from_response(Self::response(url.as_str()).await?, tmp);

        if let Some(pb) = pb {
            response = response.with_pb(pb);
        }

        Ok(response)
    }

    pub async fn response<T: IntoUrl>(url: T) -> Result<Response, DownloadFileError> {
        // default client
        let client = BiliApi::client();
        let response = client.get(url).send().await?;
        Ok(response)
    }

    pub fn progress_bar(&mut self, part: &str) -> &mut Self {
        let Some(pg) = self.pb.as_mut() else {
            return self;
        };

        pg.set_message(format!("{:<10}", part));
        pg.set_style(
            ProgressStyle::with_template(BAR_TEMPLATE)
                .unwrap()
                .progress_chars("#>-"),
        );

        self
    }

    pub fn set_bg(&mut self, dpb: DownloadProgressBar) -> &mut Self {
        let _ = self.pb.insert(dpb);

        self
    }

    pub fn with_bg(mut self, dpb: DownloadProgressBar) -> Self {
        self.set_bg(dpb);
        self
    }

    pub fn spawn_handle<F, O, T>(self, func: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce(Self) -> O + Send + 'static,
        O: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(async move { func(self).await })
    }

    pub fn into_ecs_handle_reopen<T: Into<Cow<'static, str>> + Send + 'static>(
        self,
        name: T,
    ) -> ECSHandleResult<TempFileReopen, DownloadFileError> {
        let file_icon = self.spawn_handle(move |pending| async move {
            pending
                .into_response()
                .await?
                .download_into_file_pendding()
                .await
                .and_then(|file| Ok(file.reopen()?))
                .map(|file| TempFileReopen::new(name, file))
        });

        ECSHandleResult::new(file_icon)
    }
}

#[derive(Debug)]
pub struct DownloadFileResponse {
    pub pb: Option<DownloadProgressBar>,
    pub response: Response,
    pub tmp: TempFilePendding,
}

impl DownloadFileResponse {
    pub fn map_pb<F: FnOnce(&DownloadProgressBar, &Response, &TempFilePendding)>(
        self,
        func: F,
    ) -> Self {
        let Self {
            pb: pg,
            response,
            tmp,
            ..
        } = &self;
        if let Some(pg) = pg.as_ref() {
            func(pg, response, tmp);
        }
        self
    }

    pub fn map<F: FnOnce(&Self)>(self, func: F) -> Self {
        func(&self);
        self
    }

    pub fn with_pb(mut self, pb: DownloadProgressBar) -> Self {
        self.pb = Some(pb);
        self
    }

    pub fn try_headers_size(&self) -> Result<u64> {
        let hv = &self.response.headers()[CONTENT_LENGTH];
        let str = hv.to_str()?;
        let size = str.parse()?;

        Ok(size)
    }

    pub fn by_task(self, runtimer: &mut TokioTasksRuntime) -> DownloadFileTask {
        DownloadFileTask::new(self, runtimer)
    }

    pub fn into_task(self) -> DownloadFileTask {
        let handle = tokio::spawn(async move { self.download_into_file_pendding().await });
        DownloadFileTask(ECSHandleResult::new(handle))
    }

    pub fn from_response(response: Response, tmp: TempFilePendding) -> Self {
        Self {
            pb: None,
            tmp,
            response,
        }
    }

    pub async fn from_url<T: IntoUrl>(
        url: T,
        tmp: TempFilePendding,
    ) -> Result<Self, DownloadFileError> {
        let response = DownloadFilePending::response(url).await?;

        Ok(Self {
            pb: None,
            tmp,
            response,
        })
    }

    pub fn response(&self) -> &Response {
        &self.response
    }

    pub fn with_mut<F: Fn(&mut Self)>(mut self, func: F) -> Self {
        func(&mut self);

        self
    }

    pub async fn with_download<
        F: FnOnce(Self) -> O,
        O: Future<Output = Result<Self, DownloadFileError>>,
    >(
        self,
        func: F,
    ) -> Result<Self, DownloadFileError> {
        func(self).await
    }

    pub async fn download(self) -> Result<Self, DownloadFileError> {
        let Self {
            pb,
            mut tmp,
            mut response,
        } = self;

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    tmp.write_all(&chunk)?;
                    tmp.flush()?;

                    if let Some(pb) = pb.as_ref() {
                        pb.inc(chunk.len() as u64);
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(DownloadFileError {
                        caller: MaybeLocation::caller(),
                        error: DownloadFileErrorKind::Reqwest(e),
                    });
                }
            }
        }

        Ok(Self { pb, response, tmp })
    }

    pub async fn download_into_file_pendding(self) -> Result<TempFilePendding, DownloadFileError> {
        let Self {
            pb: pg,
            mut tmp,
            mut response,
            ..
        } = self;

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    tmp.tmp.write_all(&chunk)?;
                    tmp.tmp.flush()?;
                    if let Some(pb) = pg.as_ref() {
                        pb.inc(chunk.len() as u64);
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(DownloadFileError {
                        caller: MaybeLocation::caller(),
                        error: DownloadFileErrorKind::Reqwest(e),
                    });
                }
            }
        }

        Ok(tmp)
    }

    pub fn spawn_handle<F, O, T>(self, func: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce(Self) -> O + Send + 'static,
        O: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(async move { func(self).await })
    }
}

#[derive(Debug, Component)]
pub struct DownloadFileTask(pub ECSHandleResult<TempFilePendding, DownloadFileError>);

impl DownloadFileTask {
    pub fn new(response: DownloadFileResponse, runtimer: &mut TokioTasksRuntime) -> Self {
        let task =
            runtimer.spawn_background_task(move |_ctx| response.download_into_file_pendding());

        let handle = ECSHandleResult::new(task);

        Self(handle)
    }

    pub async fn from_url<T: IntoUrl>(
        runtimer: &mut TokioTasksRuntime,
        url: T,
        tmp: TempFilePendding,
    ) -> Result<Self, DownloadFileError> {
        let response = DownloadFileResponse::from_url(url, tmp).await?;
        Ok(Self::new(response, runtimer))
    }
}

#[derive(Debug)]
pub struct DownloadFileError {
    pub caller: MaybeLocation,
    pub error: DownloadFileErrorKind,
}

impl DownloadFileError {
    #[track_caller]
    pub fn new(error: DownloadFileErrorKind) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error,
        }
    }
}

impl std::error::Error for DownloadFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.error {
            DownloadFileErrorKind::Reqwest(e) => Some(e),
            DownloadFileErrorKind::IO(e) => Some(e),
            DownloadFileErrorKind::SystemTime(e) => Some(e),
            DownloadFileErrorKind::ApiReq(e) => Some(e),
            DownloadFileErrorKind::Db(e) => Some(e),
            DownloadFileErrorKind::Serialize(e) => Some(e),
            DownloadFileErrorKind::Status(error) => error.source(),
            DownloadFileErrorKind::Page(e) => e.source(),
            DownloadFileErrorKind::MediaPage => None,
        }
    }
}

#[derive(Debug)]
pub enum DownloadFileErrorKind {
    Reqwest(reqwest::Error),
    IO(io::Error),
    Status(anyhow::Error),
    SystemTime(std::time::SystemTimeError),
    ApiReq(api_req::error::ApiErr),
    Db(sea_orm::DbErr),
    Page(anyhow::Error),
    Serialize(serde_json::Error),
    MediaPage,
}

impl std::fmt::Display for DownloadFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.error {
            DownloadFileErrorKind::Reqwest(error) => {
                write!(f, "Reqwest error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::IO(error) => {
                write!(f, "IO error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::Status(error) => {
                write!(f, "status error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::SystemTime(error) => {
                write!(f, "SystemTime error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::ApiReq(error) => {
                write!(f, "ApiErr error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::Db(error) => {
                write!(f, "Db error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::Page(error) => {
                write!(f, "page error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::Serialize(error) => {
                write!(f, "Serialize error:{}, caller:{:?}", error, self.caller)
            }
            DownloadFileErrorKind::MediaPage => {
                write!(
                    f,
                    "media page error, maybe media is not exist, caller:{:?}",
                    self.caller
                )
            }
        }
    }
}

impl From<io::Error> for DownloadFileError {
    #[track_caller]
    fn from(error: io::Error) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: DownloadFileErrorKind::IO(error),
        }
    }
}

impl From<reqwest::Error> for DownloadFileError {
    #[track_caller]
    fn from(error: reqwest::Error) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: DownloadFileErrorKind::Reqwest(error),
        }
    }
}

impl From<std::time::SystemTimeError> for DownloadFileError {
    #[track_caller]
    fn from(error: std::time::SystemTimeError) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: DownloadFileErrorKind::SystemTime(error),
        }
    }
}

impl From<api_req::error::ApiErr> for DownloadFileError {
    #[track_caller]
    fn from(error: api_req::error::ApiErr) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: DownloadFileErrorKind::ApiReq(error),
        }
    }
}

impl From<sea_orm::DbErr> for DownloadFileError {
    #[track_caller]
    fn from(error: sea_orm::DbErr) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: DownloadFileErrorKind::Db(error),
        }
    }
}

impl From<serde_json::Error> for DownloadFileError {
    #[track_caller]
    fn from(error: serde_json::Error) -> Self {
        Self {
            caller: MaybeLocation::caller(),
            error: DownloadFileErrorKind::Serialize(error),
        }
    }
}
