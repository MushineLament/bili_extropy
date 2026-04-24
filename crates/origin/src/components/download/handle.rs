use std::{
    borrow::Cow,
    fs::{self, File},
    hash::Hash,
    io::{self, ErrorKind, Seek as _, Write as _},
    mem,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicU64},
};

use crate::{
    api::BiliApi,
    components::{
        download::{MediaInfoAidPayload, MediaInfoBvidPayload},
        handle::{ECSHandle, ECSHandleError, ECSHandleResult},
    },
    cookies::{add_cookie_jar, parse_cookies},
    db::Db,
    entity::{
        BvId, MediaAid,
        media::{self, Media, MediaInfoData, MediaInfoResp, MediaInfoSingle, Page},
        status::StatusModel,
    },
    output::{EntryOuput, IndexAudio, IndexOuput, IndexVideo},
    payload::DashPayload,
    response::{Dash, DashData, DashResp},
    state::MediaState,
};
use anyhow::{Result, anyhow};
use api_req::{ApiCaller as _, error::ApiErr};
use bevy::{
    ecs::{change_detection::MaybeLocation, component::Component},
    platform::collections::{HashMap, HashSet, hash_map},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use migration::OnConflict;
use reqwest::{IntoUrl, Response, header::CONTENT_LENGTH};
use sea_orm::{
    ActiveValue::{Set, Unchanged},
    EntityTrait as _, IntoActiveModel as _,
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

    pub async fn response(self) -> Result<MediaInfoSingle, ApiErr> {
        let anyhow = anyhow::anyhow!("request aid media<{:?}>", self.0);

        let aid = self.0.parse::<MediaAid>();

        let response: Result<MediaInfoSingle, ApiErr> =
            BiliApi::request(MediaInfoBvidPayload { bvid: self.0 }).await;

        // first try BvId
        if let Ok(media) = response {
            return Ok(media);
        };

        let Ok(aid) = aid else {
            // if not a mediacid and not a avlid bvid
            error!(
                "{:?}",
                anyhow::anyhow!("{:?} error:{:?}", anyhow, MaybeLocation::caller())
            );
            return response;
        };

        let response: Result<MediaInfoSingle, ApiErr> =
            BiliApi::request(MediaInfoAidPayload { aid }).await;

        response
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct DownloadHandle(pub ECSHandle<Result<BvId, DownloadFileError>>);

impl DownloadHandle {
    pub fn new<T: Into<String>>(
        db: Db,
        bars: MultiProgress,
        cookies: T,
        list: DownloadWay,
        runtimer: &mut TokioTasksRuntime,
        active_status: Cow<'static, Vec<StatusModel>>,
    ) -> Self {
        let cookies = cookies.into();
        let task = async move {
            add_cookie_jar(parse_cookies(&cookies));

            let bvid = list.0.clone();

            let media = list
                .response()
                .await
                .map_err(|err| DownloadFileError::new(DownloadFileErrorKind::ApiReq(err)));

            let Ok(MediaInfoSingle {
                code: _,
                data: Some(media),
                message: _,
            }) = media
            else {
                return media.map(|_| bvid);
            };

            let aid = media.aid;
            let bvid = media.bvid.clone();

            let model = crate::entity::media::MediaModel {
                aid: media.aid,
                bv_id: media.bvid.to_owned(),
                title: media.title.to_owned(),
                r#type: media.r#type.to_string(),
                state: MediaState::Pending.to_string(),
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
                                media::Column::State,
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
            )
            .await
            .map(|_| bvid);

            info!("task finished");
            result
        };

        let handle = runtimer.spawn_background_task(move |_ctx| task);
        Self(ECSHandle::new(handle))
    }
}

pub async fn download(
    db: &Db,
    media: &Media,
    bars: MultiProgress,
    tmp_path: &Path,
    active_status: &[StatusModel],
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
        .map(|model| Path::new(&model.path).join(&model.name))
        .filter(|path| !path.exists() && fs::create_dir_all(path).is_ok())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    let system_time = std::time::SystemTime::now();
    let duration_file_create = system_time.duration_since(std::time::UNIX_EPOCH)?;

    let mut file_entry_json = EntryOuput::default();
    file_entry_json.time_update_stamp = duration_file_create.as_millis();
    file_entry_json.update_media(media);

    let total_bytes = Arc::new(AtomicU64::new(0));

    let mut index = IndexOuput::default();

    let file_icon = DownloadFilePending::from_tmp_url(tmp_path, media.pic.as_str())?.spawn_handle(
        |pending| async move {
            pending
                .into_response()
                .await?
                .task()
                .await
                .and_then(|file| Ok(file.reopen()?))
                .map(|file| TempFileReopen::new("cover.jpg", file))
        },
    );

    let mut handle_icon: ECSHandleResult<TempFileReopen, DownloadFileError> =
        ECSHandleResult::new(file_icon);

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
            state: Set(match code {
                -403 | 62012 | 62002 => MediaState::PermissionDenied,
                _ => MediaState::Expired,
            }
            .to_string()),
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

        let video_size = total_bytes.clone();
        let video_pb = pb.clone();
        let file_video = v
            .as_ref()
            .and_then(|v| {
                file_entry_json.update_video(v);
                index_video.update_video(v);
                DownloadFilePending::from_tmp_url(tmp_path, v.base_url.as_str()).ok()
            })
            .map(move |pending| {
                pending
                    .with_bg(video_pb.clone())
                    .spawn_handle(|pending| async move {
                        pending
                            .into_response()
                            .await?
                            .with(|res| {
                                if let Ok(size) = res.try_headers_size() {
                                    video_size
                                        .fetch_add(size, std::sync::atomic::Ordering::Relaxed);
                                    res.pg.as_ref().map(|pg| pg.inc_length(size));
                                }
                            })
                            .task()
                            .await
                            .and_then(|file| Ok(file.reopen()?))
                            .map(|file| TempFileReopen::new("video.m4s", file))
                    })
            });

        let audio_size = total_bytes.clone();
        let audio_pb = pb.clone();
        let file_audio = a
            .as_ref()
            .and_then(|a| {
                file_entry_json.update_audio(a);
                index_audio.update_audio(a);
                DownloadFilePending::from_tmp_url(tmp_path, a.base_url.as_str()).ok()
            })
            .map(move |pending| {
                pending
                    .with_bg(audio_pb.clone())
                    .spawn_handle(|pending| async move {
                        pending
                            .into_response()
                            .await?
                            .with(|res| {
                                if let Ok(size) = res.try_headers_size() {
                                    audio_size
                                        .fetch_add(size, std::sync::atomic::Ordering::Relaxed);
                                    res.pg.as_ref().map(|pg| pg.inc_length(size));
                                }
                            })
                            .task()
                            .await
                            .and_then(|file| Ok(file.reopen()?))
                            .map(|file| TempFileReopen::new("audio.m4s", file))
                    })
            });

        index_video.update_audio_id(index_audio.audio_id);

        let file_danmu = {
            let danmu_url = format!(
                "https://api.bilibili.com/x/v2/dm/web/seg.so?type=1&oid={}&segment_index=1",
                media.cid
            );
            DownloadFilePending::from_tmp_url(tmp_path, danmu_url)?
                .into_response()
                .await?
                .task()
                .await?
                .reopen()
                .map(|file| TempFileReopen::new("danmaku.pb", file))
        };

        let file_danmu_xml = {
            let xml_url = format!("https://api.bilibili.com/x/v1/dm/list.so?oid={}", media.cid);

            let http_response = DownloadFilePending::response(xml_url).await?;

            let bytes = http_response.bytes().await?;

            // 解压 deflate 数据
            let mut decoder = flate2::bufread::DeflateDecoder::new(&bytes[..]);
            let mut decompressed = Vec::new();
            use std::io::Read;

            decoder.read_to_end(&mut decompressed)?;

            let mut file_danmu_xml = NamedTempFile::new_in(tmp_path)?;

            file_danmu_xml.write_all(&decompressed)?;

            file_danmu_xml
                .reopen()
                .map(|file| TempFileReopen::new("danmaku.xml", file))
        };

        file_entry_json.downloaded_bytes = pb.position();

        file_entry_json.page_data.from = "vupload".to_string();
        file_entry_json.is_completed = true;

        let duration_file_update = system_time.duration_since(std::time::UNIX_EPOCH)?;
        file_entry_json.time_create_stamp = duration_file_update.as_millis();

        file_entry_json.total_bytes = total_bytes.load(std::sync::atomic::Ordering::Relaxed);

        let file_entry = {
            let file_entry = NamedTempFile::new_in(tmp_path)?;

            let entry_writer = io::BufWriter::new(&file_entry);
            serde_json::to_writer_pretty(entry_writer, &file_entry_json)?; // 使用 pretty 格式化输出;

            file_entry
                .reopen()
                .map(|file| TempFileReopen::new("entry.json", file))
        };

        let mut info_files = vec![file_danmu, file_danmu_xml, file_entry];

        if let Ok(file_icon) = handle_icon.block_on() {
            info_files.push(file_icon.try_clone())
        };

        // 视频id目录下
        for folder in folders
            .iter()
            .map(|folder| folder.join(media.aid.to_string()))
            .map(|aid_foler| aid_foler.join(format!("c_{}", cid.to_string())))
        {
            let files = info_files
                .iter()
                .filter_map(|file| file.as_ref().ok())
                .map(|file| file.try_clone())
                .filter_map(|file| file.ok());

            tmptofolder.add_path_and_files(folder, files);
        }

        let mut medias_files = vec![];

        if let Some(file_video) = file_video
            && let Ok(file_video) = file_video.await
        {
            if let Ok(file_video) = file_video.as_ref()
                && let Ok((size, md5)) = file_video.get_size_md5()
            {
                index_video.update_md5_size(md5, size);
            }

            medias_files.push(file_video);
        }

        if let Some(file_audio) = file_audio
            && let Ok(file_audio) = file_audio.await
        {
            if let Ok(file_audio) = file_audio.as_ref()
                && let Ok((size, md5)) = file_audio.get_size_md5()
            {
                index_audio.update_md5_size(md5, size);
            }

            medias_files.push(file_audio);
        }

        // index.json
        let file_index = NamedTempFile::new_in(tmp_path)?;

        let index_writer = io::BufWriter::new(&file_index);

        index.video.push(index_video);
        index.audio.push(index_audio);

        serde_json::to_writer_pretty(index_writer, &index)?; // 使用 pretty 格式化输出;

        let file_index = file_index
            .reopen()
            .map(|file| TempFileReopen::new("index.json", file))
            .map_err(|err| DownloadFileError::new(DownloadFileErrorKind::IO(err)));

        medias_files.push(file_index);

        // 视频清晰度目录下
        for folder in folders
            .iter()
            .map(|folder| folder.join(media.aid.to_string()))
            .map(|aid_foler| aid_foler.join(format!("c_{}", cid.to_string())))
            .map(|up_cid| up_cid.join(v.as_ref().map(|v| v.id.to_string()).unwrap_or_default()))
        {
            let files = medias_files
                .iter()
                .filter_map(|file| file.as_ref().ok())
                .map(|file| file.try_clone())
                .filter_map(|file| file.ok());

            tmptofolder.add_path_and_files(folder, files);
        }
    }

    tmptofolder.build();

    media::MediaEntity::update(media::ActiveModel {
        aid: Unchanged(media.aid),
        state: Set(MediaState::Completed.to_string()),
        ..Default::default()
    })
    .exec(&db.db)
    .await?;

    Ok(())
}

#[derive(Debug)]
pub struct TempFileReopen {
    pub name: Cow<'static, str>,
    /// PartialEq, Eq, Hash will ignore this,
    pub tmp: File,
}

impl TempFileReopen {
    pub fn new<T: Into<Cow<'static, str>>>(name: T, tmp: File) -> Self {
        Self {
            name: name.into(),
            tmp,
        }
    }

    pub fn try_clone(&self) -> Result<Self, io::Error> {
        Ok(Self {
            name: self.name.clone(),
            tmp: self.tmp.try_clone()?,
        })
    }

    pub fn get_size_md5(&self) -> Result<(u64, String), io::Error> {
        let mut file = self.tmp.try_clone()?;

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
        let meta = self.tmp.metadata()?;
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
                if let Err(e) = file_name.tmp.seek(io::SeekFrom::Start(0)) {
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

                match io::copy(&mut file_name.tmp, &mut target_file) {
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
    pub pg: Option<DownloadProgressBar>,
    pub tmp: NamedTempFile,
    pub url: Url,
}

impl DownloadFilePending {
    pub fn new<T: IntoUrl>(url: T, tmp: NamedTempFile) -> Result<Self, DownloadFileError> {
        Ok(Self {
            pg: None,
            url: url.into_url()?,
            tmp,
        })
    }

    #[track_caller]
    pub fn from_tmp_url<P: AsRef<Path>, T: IntoUrl>(
        dir: P,
        url: T,
    ) -> Result<Self, DownloadFileError> {
        let tmp = NamedTempFile::new_in(dir)?;
        let url = url.into_url()?;

        Ok(Self { pg: None, tmp, url })
    }

    pub fn new_with_bg<T: IntoUrl>(
        url: T,
        tmp: NamedTempFile,
        pg: DownloadProgressBar,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            pg: Some(pg),
            tmp,
            url: url.into_url()?,
        })
    }

    pub async fn into_response(self) -> Result<DownloadFileResponse, DownloadFileError> {
        let Self { pg, tmp, url } = self;

        let response = Self::response(url.as_str()).await?;

        Ok(DownloadFileResponse { pg, tmp, response })
    }

    pub async fn response<T: IntoUrl>(url: T) -> Result<Response, DownloadFileError> {
        // default client
        let client = BiliApi::client();
        let response = client.get(url).send().await?;
        Ok(response)
    }

    pub fn progress_bar(&mut self, part: &str) -> &mut Self {
        let Some(pg) = self.pg.as_mut() else {
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
        let _ = self.pg.insert(dpb);

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
}

#[derive(Debug)]
pub struct DownloadFileResponse {
    pub pg: Option<DownloadProgressBar>,
    pub response: Response,
    pub tmp: NamedTempFile,
}

impl DownloadFileResponse {
    pub fn with_pb<F: FnOnce(&DownloadProgressBar, &Response, &NamedTempFile)>(
        self,
        func: F,
    ) -> Self {
        let Self { pg, response, tmp } = &self;
        if let Some(pg) = pg.as_ref() {
            func(pg, response, tmp);
        }
        self
    }

    pub fn with<F: FnOnce(&Self)>(self, func: F) -> Self {
        func(&self);
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
        let handle = tokio::spawn(async move { self.task().await });
        DownloadFileTask(ECSHandleResult::new(handle))
    }

    pub async fn from_url<T: IntoUrl>(
        url: T,
        tmp: NamedTempFile,
    ) -> Result<Self, DownloadFileError> {
        let response = DownloadFilePending::response(url).await?;

        Ok(Self {
            pg: None,
            tmp,
            response,
        })
    }

    pub fn response(&self) -> &Response {
        &self.response
    }

    pub async fn task(self) -> Result<NamedTempFile, DownloadFileError> {
        let Self {
            pg,
            mut tmp,
            mut response,
        } = self;

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    tmp.write_all(&chunk)?;
                    tmp.flush()?;
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
pub struct DownloadFileTask(pub ECSHandleResult<NamedTempFile, DownloadFileError>);

impl DownloadFileTask {
    pub fn new(response: DownloadFileResponse, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = runtimer.spawn_background_task(move |_ctx| response.task());

        let handle = ECSHandleResult::new(task);

        Self(handle)
    }

    pub async fn from_url<T: IntoUrl>(
        runtimer: &mut TokioTasksRuntime,
        url: T,
        tmp: NamedTempFile,
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
}

impl std::fmt::Display for DownloadFileError {
    #[track_caller]
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
