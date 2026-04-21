use std::{
    borrow::Cow,
    fs::{self, File},
    hash::Hash,
    io::{self, Read as _, Write as _},
    mem,
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr,
    sync::Arc,
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
use reqwest::{
    IntoUrl, Response,
    header::{CONTENT_LENGTH, HeaderValue},
};
use sea_orm::{
    ActiveValue::{Set, Unchanged},
    EntityTrait as _, IntoActiveModel as _,
};
use tempfile::NamedTempFile;
use tracing::{error, info, warn};
use url::Url;

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
    response::{self, Dash, DashData, DashResp},
    state::MediaState,
};

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
    response: &Media,
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
                        "create directoy error: {:?},file: {:?}, line: {:?}",
                        err,
                        file!(),
                        line!()
                    )
                }
                return false;
            }

            if !path.is_dir() {
                error!(
                    "The path is not a directory,file: {:?}, line: {:?}",
                    file!(),
                    line!()
                );

                return false;
            }

            true
        })
        .collect::<Vec<_>>();

    let file_icon = DownloadFile::task(
        response.pic.clone(),
        NamedTempFile::new_in(tmp_path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })?,
        None,
    )
    .await?;

    let MediaInfoResp {
        code,
        data,
        message,
    } = BiliApi::request(MediaInfoAidPayload { aid: response.aid })
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
            cid: _media_cid,
        }),
        0,
    ) = (data, code)
    else {
        media::MediaEntity::update(media::ActiveModel {
            aid: Unchanged(response.aid),
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
            response.aid,
            response.title,
            message.unwrap_or_default()
        ));
    };

    let only1p = pages.len() == 1;
    for Page { cid, page, part } in pages {
        let _filename = if only1p {
            format!("{}-{}", response.aid, response.title)
        } else {
            format!("{}-{}({page})-{part}", response.aid, response.title)
        };
        let DashResp {
            data:
                DashData {
                    dash: Dash { video, audio },
                    timelength,
                },
            ..
        } = BiliApi::request(DashPayload::new(response.aid, cid).await.map_err(|err| {
            anyhow::anyhow!(
                "caller: {:?},aid:{:?},get response error:{:?}",
                (file!(), line!()),
                response.aid,
                err,
            )
        })?)
        .await
        .map_err(|_err| {
            anyhow!(
                "caller: {:?},get dash payload err,maybe is pay media or other reason,bvid: {:?}",
                (file!(), line!()),
                response.bvid,
            )
        })?;

        let hv2u64 = |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };

        let mut video = video.into_iter();
        let mut audio = audio.into_iter();

        let (v, a) = (video.next(), audio.next());

        if v.is_none() && a.is_none() {
            // TODO! need more detailed error infomation.
            continue;
        }

        let pb = DownloadProgressBar::default();
        pb.progress_bar(&bars, &part);

        let file_video = match v.as_ref() {
            Some(v) => {
                let response = BiliApi::client().get(v.base_url.as_str()).send().await?;
                let size = hv2u64(&response.headers()[CONTENT_LENGTH]);
                pb.inc_length(size);

                let tmp = NamedTempFile::new_in(tmp_path).map_err(|err| {
                    anyhow::anyhow!(
                        "can't not download temp file err: {:?},caller: {:?}",
                        err,
                        MaybeLocation::caller()
                    )
                })?;
                let tmp = DownloadFile::task_by_response(response, tmp, Some(pb.clone())).await?;

                let file_video = tmp
                    .reopen()
                    .map(|file| TempFileToName::new("video.m4s", file))?;

                Some(file_video)
            }
            _ => None,
        };

        let file_audio = match a.as_ref() {
            Some(a) => {
                let response = BiliApi::client().get(a.base_url.as_str()).send().await?;
                let size = hv2u64(&response.headers()[CONTENT_LENGTH]);

                pb.inc_length(size);

                let tmp = NamedTempFile::new_in(tmp_path).map_err(|err| {
                    anyhow::anyhow!(
                        "can't not download temp file err: {:?},caller: {:?}",
                        err,
                        MaybeLocation::caller()
                    )
                })?;

                let tmp = DownloadFile::task_by_response(response, tmp, Some(pb.clone())).await?;

                let file_audio = tmp
                    .reopen()
                    .map(|file| TempFileToName::new("audio.m4s", file))?;

                Some(file_audio)
            }
            _ => None,
        };

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
                response.cid
            );

            DownloadFile::task(danmu_url, file_danmu, None)
                .await?
                .reopen()
                .map(|file| TempFileToName::new("danmaku.pb", file))
        };

        let file_danmu_xml = {
            let xml_url = format!(
                "https://api.bilibili.com/x/v1/dm/list.so?oid={}",
                response.cid
            );

            let http_response = DownloadFile::response(xml_url).await?;

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

            let mut file_danmu_xml = NamedTempFile::new_in(tmp_path).map_err(|err| {
                anyhow::anyhow!(
                    "can't not download temp file err: {:?},caller: {:?}",
                    err,
                    MaybeLocation::caller()
                )
            })?;

            file_danmu_xml.write_all(&decompressed)?;

            file_danmu_xml
                .reopen()
                .map(|file| TempFileToName::new("danmaku.xml", file))
        };

        let file_entry = NamedTempFile::new_in(tmp_path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                MaybeLocation::caller()
            )
        })?;

        let system_now = std::time::SystemTime::now();

        let duration_file_create =
            system_now
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|err| {
                    anyhow::anyhow!(
                        "get system time err: {:?},caller: {:?}",
                        err,
                        MaybeLocation::caller()
                    )
                })?;

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

        let mut index = IndexOuput::default();

        let audio_id = if let Some(a) = a
            && let Some(file_a) = file_audio.as_ref()
        {
            let (a_size, a_hex) = file_a.get_size_md5()?;
            index.audio.push(IndexAudio {
                id: a.id,
                base_url: a.base_url.clone(),
                backup_url: a.backup_url.clone(),
                bandwidth: a.bandwidth,
                codecid: a.codecid,
                md5: a_hex,
                size: a_size,
                audio_id: 0,
                no_rexcode: false,
                frame_rate: a.frame_rate.clone(),
                width: a.width,
                height: a.height,
                widevine_pssh: "".to_string(),
                bilidrm_uri: "".to_string(),
            });
            a.id
        } else {
            0
        };

        let v_id = if let Some(v) = v.as_ref()
            && let Some(file_v) = file_video.as_ref()
        {
            let (v_size, v_hex) = file_v.get_size_md5()?;
            index.video.push(IndexVideo {
                id: v.id.clone(),
                base_url: v.base_url.clone(),
                backup_url: v.backup_url.clone(),
                bandwidth: v.bandwidth,
                codecid: v.codecid,
                md5: v_hex,
                size: v_size,
                audio_id: audio_id,
                no_rexcode: false,
                frame_rate: v.frame_rate.clone(),
                width: v.width,
                height: v.height,
                widevine_pssh: "".to_string(),
                bilidrm_uri: "".to_string(),
            });
            v.id
        } else {
            0
        };

        let index_writer = io::BufWriter::new(&file_index);
        serde_json::to_writer_pretty(index_writer, &index)
            .context("Can't upsert into index.json")?; // 使用 pretty 格式化输出;

        let entry = EntryOuput {
            media_type: response.r#type.clone() as i64,
            has_dash_audio: audio_id != 0,
            is_completed: true,
            total_bytes: file_audio
                .as_ref()
                .and_then(|a| a.size().ok())
                .unwrap_or_default()
                + file_video
                    .as_ref()
                    .and_then(|v| v.size().ok())
                    .unwrap_or_default(),
            downloaded_bytes: file_audio
                .as_ref()
                .and_then(|a| a.size().ok())
                .unwrap_or_default()
                + file_video
                    .as_ref()
                    .and_then(|v| v.size().ok())
                    .unwrap_or_default(),
            title: response.title.clone(),
            type_tag: v_id.to_string(),
            cover: response.pic.clone(),
            video_quality: v_id,
            prefered_video_quality: v_id,
            guessed_total_bytes: 0,
            total_time_milli: timelength,
            //https://github.com/ILoveScratch2/bilibili-api-collect-new/blob/main/docs/danmaku/danmaku_view_proto.md
            danmaku_count: -1,
            time_update_stamp: duration_file_create.as_millis(),
            time_create_stamp: duration_file_update.as_millis(),
            can_play_in_advance: true,
            interrupt_transform_temp_file: false,
            quality_pithy_description: match v_id {
                6 => "240P",
                16 => "360P",
                32 => "480P",
                64 => "720P",
                74 => "720P60",
                80 => "1080P",
                100 => "智能修复",
                112 => "1080P+",
                116 => "1080P60",
                120 => "4K",
                125 => "HDR",
                126 => "杜比视界",
                127 => "8K",
                129 => "HDR",
                _ => unreachable!("未定义的视频清晰度:{:?}", v_id),
            }
            .to_string(),
            quality_superscript: "".to_owned(),
            variable_resolution_ratio: false,
            cache_version_code: -1,
            preferred_audio_quality: 0,
            audio_quality: 0,
            avid: response.aid,
            spid: 0,
            season_id: 0,
            bvid: "".to_string(),
            owner_id: owner.mid,
            owner_name: owner.name.clone(),
            is_charge_video: false,
            verification_code: 0,
            page_data: EntryPageData {
                cid,
                page,
                from: "vupload".to_string(),
                part: response.title.clone(),
                link: format!("bilibili://video/{}?cid={}", response.aid, cid),
                rich_vid: "".to_owned(),
                has_alias: false,
                tid: 0,
                width: v.as_ref().map(|v| v.width).unwrap_or_default(),
                height: v.as_ref().map(|v| v.height).unwrap_or_default(),
                rotate: 0,
                download_title: "".to_owned(),
                download_subtitle: "".to_owned(),
            },
            ep: None,
        };

        let entry_writer = io::BufWriter::new(&file_entry);
        serde_json::to_writer_pretty(entry_writer, &entry)
            .context("Can't upsert into index.json")?; // 使用 pretty 格式化输出;

        let mut tmptofolder = FilesIntoFolders::new();

        let file_entry = file_entry
            .reopen()
            .map(|file| TempFileToName::new("entry.json", file));

        let file_icon = file_icon
            .reopen()
            .map(|file| TempFileToName::new("cover.jpg", file));

        let info_files = [file_danmu, file_danmu_xml, file_entry, file_icon];

        // 视频id目录下
        for folder in folders
            .iter()
            .map(|folder| folder.join(response.aid.to_string()))
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
            .map(|file| TempFileToName::new("index.json", file))
            .map_err(|err| {
                error!("TODO!:{:?}", err);
            })
            .ok();

        let medias_files = [file_video, file_audio, file_index];

        // 视频清晰度目录下
        for folder in folders
            .iter()
            .map(|folder| folder.join(response.aid.to_string()))
            .map(|aid_foler| aid_foler.join(format!("c_{}", cid.to_string())))
            .map(|up_cid| up_cid.join(v_id.to_string()))
        {
            let files = medias_files
                .iter()
                .filter_map(|file| file.as_ref())
                .map(|file| file.try_clone())
                .filter_map(|file| file.ok());

            tmptofolder.add_path_and_files(folder, files);
        }

        tmptofolder.build();
    }

    media::MediaEntity::update(media::ActiveModel {
        aid: Unchanged(response.aid),
        state: Set(MediaState::Completed.to_string()),
        ..Default::default()
    })
    .exec(&db.db)
    .await?;

    Ok(())
}

#[derive(Debug)]
pub struct TempFileToName {
    pub name: Cow<'static, str>,
    /// PartialEq, Eq, Hash will ignore this,
    pub tmp: File,
}

impl TempFileToName {
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

impl PartialEq for TempFileToName {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for TempFileToName {}
impl Hash for TempFileToName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
pub struct FilesIntoFolders(pub HashMap<PathBuf, HashSet<TempFileToName>>);

impl FilesIntoFolders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_path_and_files<
        P: Into<PathBuf>,
        I: IntoIterator<Item = T>,
        T: Into<TempFileToName>,
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

                if let Err(err) = io::copy(&mut file_name.tmp, &mut target_file) {
                    error!(
                        "copy temp file to target path<{:?}> error:{:?}",
                        target_path, err
                    );
                }
            }

            name.extend(to_vec);
        }

        self.clear();
    }
}

#[derive(Debug, Deref, DerefMut, Clone)]
pub struct DownloadProgressBar(pub Cow<'static, ProgressBar>);

impl DownloadProgressBar {
    pub fn new(len: u64) -> Self {
        Self(Cow::Owned(ProgressBar::new(len)))
    }

    pub fn progress_bar(&self, bars: &MultiProgress, part: &str) -> &Self {
        bars.add(self.as_ref().clone());

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

#[derive(Debug, Component, Deref, DerefMut)]
pub struct DownloadFile(pub DbHandleResult<NamedTempFile, anyhow::Error>);

impl DownloadFile {
    pub fn new<T: IntoUrl + Send + 'static>(
        runtimer: &mut TokioTasksRuntime,
        url: T,
        tmp: NamedTempFile,
    ) -> Self {
        let task = Self::task(url, tmp, None);
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(DbHandleResult::new(handle))
    }

    pub fn new_with_bg<T: IntoUrl + Send + 'static>(
        runtimer: &mut TokioTasksRuntime,
        url: T,
        tmp: NamedTempFile,
        pb: DownloadProgressBar,
    ) -> Self {
        let task = Self::task(url, tmp, Some(pb));
        let handle = runtimer.spawn_background_task(|_ctx| task);
        Self(DbHandleResult::new(handle))
    }

    pub async fn task<T: IntoUrl>(
        url: T,
        tmp: NamedTempFile,
        pb: Option<DownloadProgressBar>,
    ) -> Result<NamedTempFile> {
        let response = Self::response(url).await?;
        Self::task_by_response(response, tmp, pb).await
    }

    pub async fn response<T: IntoUrl>(url: T) -> Result<Response> {
        // default client
        let client = BiliApi::client();
        let response = client.get(url).send().await?;
        Ok(response)
    }

    pub async fn task_by_response(
        mut response: reqwest::Response,
        mut tmp: NamedTempFile,
        pb: Option<DownloadProgressBar>,
    ) -> Result<NamedTempFile> {
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
                    return Err(anyhow!("Failed to pic: {e}"));
                }
            }
        }

        Ok(tmp)
    }
}
