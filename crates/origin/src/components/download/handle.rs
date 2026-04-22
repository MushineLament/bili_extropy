use std::{
    borrow::Cow,
    fs::{self, File},
    hash::Hash,
    io::{self, Seek as _, Write as _},
    mem,
    path::{Path, PathBuf},
};

use crate::{
    api::BiliApi,
    components::{
        download::{MediaInfoAidPayload, MediaInfoBvidPayload},
        handle::{DbHandle, DbHandleResult},
    },
    cookies::{add_cookie_jar, parse_cookies},
    db::Db,
    entity::{
        BvId, MediaAid,
        media::{self, Media, MediaInfoData, MediaInfoResp, MediaInfoSingle, Page},
        status::StatusModel,
    },
    output::{EntryOuput, EntryPageData, IndexAudio, IndexOuput, IndexVideo},
    payload::DashPayload,
    response::{Dash, DashData, DashResp},
    state::MediaState,
};
use anyhow::{Context, Result, anyhow};
use api_req::{ApiCaller as _, error::ApiErr};
use bevy::{
    ecs::{change_detection::MaybeLocation, component::Component},
    platform::collections::{HashMap, HashSet, hash_map},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use futures::TryFutureExt;
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

    pub async fn response(self) -> Result<MediaInfoSingle> {
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
            return Err(anyhow::anyhow!(
                "{:?} error:{:?}",
                anyhow,
                MaybeLocation::caller()
            ));
        };

        let response: Result<MediaInfoSingle, ApiErr> =
            BiliApi::request(MediaInfoAidPayload { aid }).await;

        response.map_err(|_err| anyhow::anyhow!("{:?} error:{:?}", anyhow, MaybeLocation::caller()))
    }
}

#[derive(Debug, Component, Deref, DerefMut)]
pub struct DownloadHandle(pub DbHandle<Result<BvId>>);

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

            let info_err = anyhow!("Info unreachable media<{}> :", cookies);

            let media = list.response().await?;

            let MediaInfoSingle {
                code: _,
                data: Some(media),
                message: _,
            } = media
            else {
                return Err(anyhow!("{:?} not has media infomation", info_err));
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

            let prikey =
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
                    .await;

            prikey.map_err(|err| {
                anyhow!(
                    "can't not upsert media<{:?}> in to table, error:{:?}",
                    aid,
                    err
                )
            })?;

            download(
                &db,
                &media,
                bars.clone(),
                &Path::new(TEMP_DOWNLOAD_FOLDER),
                active_status.as_ref(),
            )
            .await?;

            info!("task finished");
            Ok(bvid)
        };

        let handle = runtimer.spawn_background_task(move |_ctx| task);
        Self(DbHandle::new(handle))
    }
}

pub async fn download(
    db: &Db,
    media: &Media,
    bars: MultiProgress,
    tmp_path: &Path,
    active_status: &[StatusModel],
) -> Result<()> {
    if !Path::new(tmp_path).exists() {
        if let Err(err) = fs::create_dir_all(tmp_path) {
            return Err(anyhow!("can't create temp download folder: {:?}", err));
        };
    }

    if !Path::new(tmp_path).is_dir() {
        return Err(anyhow!(".temp not is a folder"));
    }

    if active_status.is_empty() {
        return Err(anyhow!("Not any status error"));
    }

    let folders = active_status
        .iter()
        .map(|model| Path::new(&model.path).join(&model.name))
        .filter(|path| {
            if !path.exists() {
                if let Err(err) = fs::create_dir_all(path) {
                    error!(
                        "create directoy error: {:?},caller: {:?}",
                        err,
                        MaybeLocation::caller()
                    )
                }
                return false;
            }

            if !path.is_dir() {
                error!(
                    "The path is not a directory,caller: {:?}",
                    MaybeLocation::caller()
                );

                return false;
            }

            true
        })
        .collect::<Vec<_>>();

    let system_now = std::time::SystemTime::now();

    let duration_file_create = system_now
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| {
            anyhow::anyhow!(
                "get system time err: {:?},caller: {:?}",
                err,
                MaybeLocation::caller()
            )
        })?;

    let mut file_entry_json = EntryOuput::default();

    file_entry_json.time_update_stamp = duration_file_create.as_millis();

    let mut index = IndexOuput::default();

    file_entry_json.update_media(media);

    let file_icon = DownloadFileResponse::from_url(
        media.pic.as_str(),
        NamedTempFile::new_in(tmp_path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })?,
    )
    .await?
    .task()
    .await?;

    let MediaInfoResp {
        code,
        data,
        message,
    } = BiliApi::request(MediaInfoAidPayload { aid: media.aid })
        .map_err(|err| {
            anyhow::anyhow!(
                "can't get media infomation err: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })
        .await?;

    let (
        Some(MediaInfoData {
            owner,
            pages,
            staff: _,
            cid,
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
        .map_err(|err| {
            anyhow::anyhow!(
                "set media state err: {:?},caller: {:?}",
                err,
                MaybeLocation::caller()
            )
        })
        .await?;

        return Err(anyhow!(
            "Info unreachable media<{}-{}>: {}",
            media.aid,
            media.title,
            message.unwrap_or_default()
        ));
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
        } = BiliApi::request(DashPayload::new(media.aid, cid).await.map_err(|err| {
            anyhow::anyhow!(
                "caller: {:?},aid:{:?},get response error:{:?}",
                (file!(), line!()),
                media.aid,
                err,
            )
        })?)
        .await
        .map_err(|_err| {
            anyhow!(
                "caller: {:?},get dash payload err,maybe is pay media or other reason,bvid: {:?}",
                (file!(), line!()),
                media.bvid,
            )
        })?;

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

        let mut file_video = match v.as_ref() {
            Some(v) => {
                file_entry_json.update_video(v);

                let tmp = NamedTempFile::new_in(tmp_path).map_err(|err| {
                    anyhow::anyhow!(
                        "can't not download temp file err: {:?},caller: {:?}",
                        err,
                        MaybeLocation::caller()
                    )
                })?;
                let file_video =
                    DownloadFilePending::new_with_bg(v.base_url.as_str(), tmp, pb.clone())?
                        .into_response()
                        .await?
                        .with(|res| {
                            if let Ok(size) = res.try_headers_size() {
                                file_entry_json.total_bytes += size;
                                res.pg.as_ref().map(|pg| pg.inc_length(size));
                            }
                        })
                        .task()
                        .await?
                        .reopen()
                        .map(|file| TempFileReopen::new("video.m4s", file))?;

                let (v_size, v_hex) = file_video.get_size_md5()?;

                let index = IndexVideo::from_video(v.clone(), v_hex, v_size);

                Some((file_video, index))
            }
            _ => None,
        };

        let file_audio = match a.as_ref() {
            Some(a) => {
                if let Some((_, video)) = file_video.as_mut() {
                    video.update_audio_id(a.id);
                }

                file_entry_json.update_audio(a);

                let tmp = NamedTempFile::new_in(tmp_path).map_err(|err| {
                    anyhow::anyhow!(
                        "can't not download temp file err: {:?},caller: {:?}",
                        err,
                        MaybeLocation::caller()
                    )
                })?;

                let file_audio =
                    DownloadFilePending::new_with_bg(a.base_url.as_str(), tmp, pb.clone())?
                        .into_response()
                        .await?
                        .with(|res| {
                            if let Ok(size) = res.try_headers_size() {
                                file_entry_json.total_bytes += size;
                                res.pg.as_ref().map(|pg| pg.inc_length(size));
                            }
                        })
                        .task()
                        .await?
                        .reopen()
                        .map(|file| TempFileReopen::new("audio.m4s", file))?;

                let (a_size, a_hex) = file_audio.get_size_md5()?;
                index
                    .audio
                    .push(IndexAudio::from_audio(a.clone(), a_hex, a_size));

                Some(file_audio)
            }
            _ => None,
        };

        file_video.as_ref().map(|file| {
            info!("file_video size 1:{:?}", file.0.size());
        });

        let file_video = file_video.map(|(file_video, index_video)| {
            index.video.push(index_video);
            file_video
        });

        let file_index = NamedTempFile::new_in(tmp_path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                MaybeLocation::caller()
            )
        })?;

        let file_danmu = {
            let file_danmu = NamedTempFile::new_in(tmp_path).map_err(|err| {
                anyhow::anyhow!(
                    "can't not download temp file err: {:?},caller: {:?}",
                    err,
                    MaybeLocation::caller()
                )
            })?;

            let danmu_url = format!(
                "https://api.bilibili.com/x/v2/dm/web/seg.so?type=1&oid={}&segment_index=1",
                media.cid
            );

            DownloadFileResponse::from_url(danmu_url, file_danmu)
                .await?
                .task()
                .await?
                .reopen()
                .map(|file| TempFileReopen::new("danmaku.pb", file))
        };

        let file_danmu_xml = {
            let mut file_danmu_xml = NamedTempFile::new_in(tmp_path).map_err(|err| {
                anyhow::anyhow!(
                    "can't not download temp file err: {:?},caller: {:?}",
                    err,
                    MaybeLocation::caller()
                )
            })?;

            let xml_url = format!("https://api.bilibili.com/x/v1/dm/list.so?oid={}", media.cid);

            let http_response = DownloadFilePending::response(xml_url).await?;

            let bytes = http_response
                .bytes()
                .await
                .context("Failed to read response bytes")?;

            // 解压 deflate 数据
            let mut decoder = flate2::bufread::DeflateDecoder::new(&bytes[..]);
            let mut decompressed = Vec::new();
            use std::io::Read;

            decoder.read_to_end(&mut decompressed).map_err(|err| {
                anyhow::anyhow!(
                    "unzip danmu xml error:{:?},caller:{:?}",
                    err,
                    MaybeLocation::caller()
                )
            })?;

            file_danmu_xml.write_all(&decompressed)?;

            file_danmu_xml
                .reopen()
                .map(|file| TempFileReopen::new("danmaku.xml", file))
        };

        let file_entry = NamedTempFile::new_in(tmp_path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                MaybeLocation::caller()
            )
        })?;

        let index_writer = io::BufWriter::new(&file_index);
        serde_json::to_writer_pretty(index_writer, &index)
            .context("Can't upsert into index.json")?; // 使用 pretty 格式化输出;

        file_entry_json.downloaded_bytes = file_audio
            .as_ref()
            .and_then(|a| a.size().ok())
            .unwrap_or_default()
            + file_video
                .as_ref()
                .and_then(|v| v.size().ok())
                .unwrap_or_default();

        file_entry_json.page_data.from = "vupload".to_string();
        file_entry_json.is_completed = true;

        let duration_file_update =
            system_now
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|err| {
                    anyhow::anyhow!(
                        "get system time error: {:?},caller: {:?}",
                        err,
                        MaybeLocation::caller()
                    )
                })?;
        file_entry_json.time_create_stamp = duration_file_update.as_millis();

        let entry_writer = io::BufWriter::new(&file_entry);
        serde_json::to_writer_pretty(entry_writer, &file_entry_json)
            .context("Can't upsert into index.json")?; // 使用 pretty 格式化输出;

        let file_entry = file_entry
            .reopen()
            .map(|file| TempFileReopen::new("entry.json", file));

        let file_icon = file_icon
            .reopen()
            .map(|file| TempFileReopen::new("cover.jpg", file));

        let info_files = [file_danmu, file_danmu_xml, file_entry, file_icon];

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

        let file_index = file_index
            .reopen()
            .map(|file| TempFileReopen::new("index.json", file))
            .map_err(|err| {
                error!("TODO!:{:?}", err);
            })
            .ok();

        file_video.as_ref().map(|file| {
            info!("file_video size:{:?}", file.size());
        });

        let medias_files = [file_video, file_audio, file_index];

        // 视频清晰度目录下
        for folder in folders
            .iter()
            .map(|folder| folder.join(media.aid.to_string()))
            .map(|aid_foler| aid_foler.join(format!("c_{}", cid.to_string())))
            .map(|up_cid| up_cid.join(v.as_ref().map(|v| v.id.to_string()).unwrap_or_default()))
        {
            let files = medias_files
                .iter()
                .filter_map(|file| file.as_ref())
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

    pub fn get_size_md5(&self) -> Result<(u64, String)> {
        let mut file = self.tmp.try_clone()?;

        let size = file
            .metadata()
            .context("Can't get index temp file metadata")?
            .len();

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
    pub fn new<T: IntoUrl>(url: T, tmp: NamedTempFile) -> Result<Self, reqwest::Error> {
        Ok(Self {
            pg: None,
            url: url.into_url()?,
            tmp,
        })
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

    pub async fn into_response(self) -> Result<DownloadFileResponse> {
        let Self { pg, tmp, url } = self;

        let response = Self::response(url.as_str()).await?;

        Ok(DownloadFileResponse { pg, tmp, response })
    }

    pub async fn response<T: IntoUrl>(url: T) -> Result<Response> {
        // default client
        let client = BiliApi::client();
        let response = client.get(url).send().await?;
        Ok(response)
    }

    pub fn progress_bar(&mut self, part: &str) -> Result<&mut Self> {
        let Some(pg) = self.pg.as_mut() else {
            return Err(anyhow::anyhow!("Not has progressbar"));
        };

        pg.set_message(format!("{:<10}", part));
        pg.set_style(
            ProgressStyle::with_template(BAR_TEMPLATE)
                .unwrap()
                .progress_chars("#>-"),
        );

        Ok(self)
    }

    pub fn set_bg(&mut self, dpb: DownloadProgressBar) -> Result<&mut Self> {
        let _ = self.pg.insert(dpb);

        Ok(self)
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

    pub fn into_task(self, runtimer: &mut TokioTasksRuntime) -> DownloadFile {
        DownloadFile::new(self, runtimer)
    }

    pub async fn from_url<T: IntoUrl>(url: T, tmp: NamedTempFile) -> Result<Self> {
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

    pub async fn task(self) -> Result<NamedTempFile> {
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
                    return Err(anyhow!("Failed to download: {e}"));
                }
            }
        }

        Ok(tmp)
    }
}

#[derive(Debug, Component)]
pub struct DownloadFile(pub DbHandleResult<NamedTempFile, anyhow::Error>);

impl DownloadFile {
    pub fn new(response: DownloadFileResponse, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = runtimer.spawn_background_task(move |_ctx| response.task());

        let handle = DbHandleResult::new(task);

        Self(handle)
    }

    pub async fn from_url<T: IntoUrl>(
        runtimer: &mut TokioTasksRuntime,
        url: T,
        tmp: NamedTempFile,
    ) -> Result<Self> {
        let response = DownloadFileResponse::from_url(url, tmp).await?;
        Ok(Self::new(response, runtimer))
    }
}
