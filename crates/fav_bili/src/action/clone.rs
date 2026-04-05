use std::{
    fs::{self},
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use api_req::ApiCaller as _;
use futures::TryFutureExt;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::header::{CONTENT_LENGTH, HeaderValue};
use sea_orm::ColumnTrait as _;
use tempfile::NamedTempFile;
use tracing::error;
use url::Url;

use crate::{
    api::BiliApi,
    cookies::{add_cookie_jar, parse_cookies},
    db::{Db, db},
    entity::{account, media},
    normalization::{EntryOuput, EntryPageData, IndexAudio, IndexOuput, IndexVideo},
    payload::{DashPayload, MediaInfoAidPayload, MediaInfoBvidPayload},
    response::{Dash, DashData, DashResp, MediaInfoData, MediaInfoResp, MediaInfoSingle, Page},
    state::{AccountState, MediaState},
    table::head,
};

const BAR_TEMPLATE: &str = "{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})";

pub async fn only_download(bvid: &String) -> Result<()> {
    let db = db(false).await;
    let bars = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

    // 获取一个活跃账户
    let accounts = db
        .get_accounts_filtered(account::Column::State.eq(AccountState::Active))
        .await?;
    let account = accounts
        .first()
        .ok_or_else(|| anyhow!("No active account found. Please login first."))?;

    add_cookie_jar(parse_cookies(&account.cookies));

    let m = match BiliApi::request(MediaInfoBvidPayload { bvid: bvid.clone() }).await? {
        MediaInfoSingle {
            code,
            data,
            message,
        } => data.expect(&format!(
            "Info unreachable bvid<{}>,state<{}>: {}",
            bvid,
            code,
            message.unwrap_or_default()
        )),
    };

    let bars = bars.clone();

    let media = media::MediaModel {
        aid: m.id,
        bv_id: m.bv_id.to_owned(),
        title: m.title.to_owned(),
        r#type: m.r#type.to_string(),
        state: MediaState::Pending.to_string(),
        cid: m.cid,
    };

    db.upsert_medias([media.clone()])
        .await
        .context("can't not upsert media in to table")?;

    download(&m, db, &media, bars.clone(), &Path::new(".")).await?;

    Ok(())
}

pub async fn download(
    response: &crate::response::Media,
    db: &Db,
    media: &media::MediaModel,
    bars: MultiProgress,
    path: &Path,
) -> Result<()> {
    let file = db.get_active_status().await?;

    let folders = file
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

    let icon_file = NamedTempFile::new_in(path).map_err(|err| {
        anyhow::anyhow!(
            "can't not download temp file err: {:?},caller: {:?}",
            err,
            (file!(), line!())
        )
    })?;

    download_file(response.pic.clone(), &icon_file).await?;

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
            cid: _media_cid,
        }),
        0,
    ) = (data, code)
    else {
        db.set_media_state(
            media.aid,
            match code {
                -403 | 62012 | 62002 => MediaState::PermissionDenied,
                _ => MediaState::Expired,
            },
        )
        .await?;

        return Err(anyhow!(
            "Info unreachable media<{}-{}>: {}",
            media.aid,
            media.title,
            message.unwrap_or_default()
        ));
    };

    let only1p = pages.len() == 1;
    for Page { cid, page, part } in pages {
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
        } = BiliApi::request(DashPayload::new(media.aid, cid).await?)
            .await
            .map_err(|err| {
                anyhow!(
                    "get dash payload err: {:?},caller: {:?}",
                    err,
                    (file!(), line!())
                )
            })?;

        let mut video = video.into_iter();
        let mut audio = audio.into_iter();

        let (v, a) = (video.next(), audio.next());

        if v.is_none() && a.is_none() {
            // TODO! need more detailed error infomation.
            continue;
        }

        let resp_v = match v.as_ref() {
            Some(v) => Some(BiliApi::client().get(v.base_url.clone()).send().await),
            _ => None,
        };

        let resp_a = match a.as_ref() {
            Some(a) => Some(BiliApi::client().get(a.base_url.clone()).send().await),
            _ => None,
        };

        let hv2u64 = |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };

        let size = match resp_v.as_ref() {
            Some(v) => v
                .as_ref()
                .map(|v| hv2u64(&v.headers()[CONTENT_LENGTH]))
                .unwrap_or_default(),
            None => 0,
        } + match resp_a.as_ref() {
            Some(a) => a
                .as_ref()
                .map(|a| hv2u64(&a.headers()[CONTENT_LENGTH]))
                .unwrap_or_default(),
            None => 0,
        };

        let pb = create_progress_bar(size, &bars, &part);

        let file_v = resp_v
            .is_some()
            .then_some(NamedTempFile::new_in(path).map_err(|err| {
                anyhow::anyhow!(
                    "can't not download temp file err: {:?},caller: {:?}",
                    err,
                    (file!(), line!())
                )
            })?);

        let file_a = resp_a
            .is_some()
            .then_some(NamedTempFile::new_in(path).map_err(|err| {
                anyhow::anyhow!(
                    "can't not download temp file err: {:?},caller: {:?}",
                    err,
                    (file!(), line!())
                )
            })?);

        let file_index = NamedTempFile::new_in(path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })?;

        let mut file_danmu = NamedTempFile::new_in(path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })?;

        let mut file_danmu_xml = NamedTempFile::new_in(path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })?;

        let file_entry = NamedTempFile::new_in(path).map_err(|err| {
            anyhow::anyhow!(
                "can't not download temp file err: {:?},caller: {:?}",
                err,
                (file!(), line!())
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
                        (file!(), line!())
                    )
                })?;

        let client = BiliApi::client();

        let danmu_url = format!(
            "https://api.bilibili.com/x/v2/dm/web/seg.so?type=1&oid={}&segment_index=1",
            media.cid
        );

        let mut danmu_response = client.get(&danmu_url).send().await?;

        while let Some(chunk) = danmu_response.chunk().await.map_err(|err| {
            anyhow::anyhow!(
                "get chunk error: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })? {
            file_danmu.write_all(&chunk)?;
            file_danmu.flush()?;
        }

        let client = BiliApi::client();

        let xml_url = format!("https://api.bilibili.com/x/v1/dm/list.so?oid={}", media.cid);
        let xml_response = client.get(&xml_url).send().await.map_err(|err| {
            anyhow::anyhow!(
                "get response error: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })?;

        // 获取响应体字节
        let bytes = xml_response.bytes().await.map_err(|err| {
            anyhow::anyhow!(
                "get xml bytes error: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })?;

        // 解压 deflate 数据
        let mut decoder = flate2::bufread::DeflateDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        use std::io::Read;
        decoder.read_to_end(&mut decompressed)?;

        file_danmu_xml.write_all(&decompressed)?;

        download_video_audio(
            media.aid,
            v.as_ref(),
            a.as_ref(),
            file_v.as_ref(),
            file_a.as_ref(),
            &pb,
        )
        .await?;

        let duration_file_update =
            system_now
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|err| {
                    anyhow::anyhow!(
                        "get system time error: {:?},caller: {:?}",
                        err,
                        (file!(), line!())
                    )
                })?;

        let mut index = IndexOuput::default();

        let audio_id = if let Some(a) = a
            && let Some(file_a) = file_a.as_ref()
        {
            let (a_size, a_hex) = get_size_md5(file_a)?;
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
            && let Some(file_v) = file_v.as_ref()
        {
            let (v_size, v_hex) = get_size_md5(&file_v)?;
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
            total_bytes: file_a
                .as_ref()
                .and_then(|a| a.as_file().metadata().ok())
                .map(|a| a.len())
                .unwrap_or_default()
                + file_v
                    .as_ref()
                    .and_then(|v| v.as_file().metadata().ok())
                    .map(|v| v.len())
                    .unwrap_or_default(),
            downloaded_bytes: file_a
                .as_ref()
                .and_then(|a| a.as_file().metadata().ok())
                .map(|a| a.len())
                .unwrap_or_default()
                + file_v
                    .as_ref()
                    .and_then(|v| v.as_file().metadata().ok())
                    .map(|v| v.len())
                    .unwrap_or_default(),
            title: media.title.clone(),
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
            avid: media.aid,
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
                part: media.title.clone(),
                link: format!("bilibili://video/{}?cid={}", media.aid, cid),
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

        // 视频id目录下
        file_into_folder(
            folders
                .iter()
                .map(|folder| folder.join(media.aid.to_string()))
                .map(|aid_foler| aid_foler.join(format!("c_{}", cid.to_string()))),
            &[
                ("danmaku.pb", Some(&file_danmu)),
                ("danmaku.xml", Some(&file_danmu_xml)),
                ("entry.json", Some(&file_entry)),
                ("cover.jpg", Some(&icon_file)),
            ],
        );

        // 视频清晰度目录下
        file_into_folder(
            folders
                .iter()
                .map(|folder| folder.join(media.aid.to_string()))
                .map(|aid_foler| aid_foler.join(format!("c_{}", cid.to_string())))
                .map(|up_cid| up_cid.join(v_id.to_string())),
            &[
                ("video.m4s", file_v.as_ref()),
                ("audio.m4s", file_a.as_ref()),
                ("index.json", Some(&file_index)),
            ],
        );
    }
    db.set_media_state(media.aid, MediaState::Completed).await?;
    Ok(())
}

fn create_progress_bar(size: u64, bars: &MultiProgress, part: &str) -> ProgressBar {
    let pb = ProgressBar::new(size);

    bars.add(pb.clone());

    pb.set_message(head(&part, 10));
    pb.set_style(
        ProgressStyle::with_template(BAR_TEMPLATE)
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}

async fn download_video_audio(
    id: i64,
    v: Option<&crate::response::Video>,
    a: Option<&crate::response::Audio>,
    mut file_v: Option<&NamedTempFile>,
    mut file_a: Option<&NamedTempFile>,
    pb: &ProgressBar,
) -> Result<()> {
    let mut resp_v = match v {
        Some(v) => Some(BiliApi::client().get(v.base_url.clone()).send().await?),
        _ => None,
    };

    let mut resp_a = match a {
        Some(a) => Some(BiliApi::client().get(a.base_url.clone()).send().await?),
        _ => None,
    };

    let (mut finished_v, mut finished_a) = (false, false);
    loop {
        tokio::select! {
            res = async { resp_v.as_mut().unwrap().chunk().map_err(|err| {
                anyhow::anyhow!(
                    "get chunk error: {:?},caller: {:?}",
                    err,
                    (file!(), line!())
                )}).await
            },
            if !finished_v && resp_v.is_some() && file_v.is_some() => {
                match res {
                    Ok(Some(chunk)) => {
                        file_v.as_mut().unwrap().write_all(&chunk)?;
                        file_v.as_mut().unwrap().flush()?;
                        pb.inc(chunk.len() as u64);
                    }
                    Ok(None) => finished_v = true,
                    Err(e) => return Err(anyhow!(
                        "Failed to download video {id}: {e}"
                    ))
                }
            }

            res = async { resp_a.as_mut().unwrap().chunk().map_err(|err| {
                anyhow::anyhow!(
                    "get chunk error: {:?},caller: {:?}",
                    err,
                    (file!(), line!())
                )
            }).await },
            if !finished_a && resp_a.is_some() && file_a.is_some() => {
                match res {
                    Ok(Some(chunk)) => {
                        file_a.as_mut().unwrap().write_all(&chunk)?;
                        file_a.as_mut().unwrap().flush()?;
                        pb.inc(chunk.len() as u64);
                    }
                    Ok(None) => finished_a = true,
                    Err(e) => return Err(anyhow!(
                        "Failed to download audio {id}: {e}"
                    ))
                }
            }

            else => break,
        }
    }
    pb.finish();

    Ok(())
}

/// 后续计划，实现错误收集，以便支持自行处理
/// 将 NamedTempFile 移动到指定目录下，并重命名指定文件名
fn file_into_folder<'a>(
    folders: impl IntoIterator<Item = PathBuf>,
    iter: &[(&'a str, Option<&'a NamedTempFile>)],
) {
    for folder in folders {
        if !folder.exists() {
            if let Err(err) = fs::create_dir_all(&folder) {
                error!("folder:{:?},err:{:?}", folder, err);
            };
        }

        if !folder.is_dir() {
            error!("path not is a directoy:{:?}", folder);
            continue;
        }

        for (file_name, metadata) in iter {
            let Some(metadata) = metadata else {
                continue;
            };
            if let Err(err) = fs::copy(metadata, folder.join(file_name)) {
                error!(
                    "file_name: {:?},path: {:?},err: {:?}",
                    folder, file_name, err
                );
            }
        }
    }
}

fn get_size_md5(file: &NamedTempFile) -> Result<(u64, String)> {
    let size = file
        .as_file()
        .metadata()
        .context("Can't get index temp file metadata")?
        .len();

    let mut v_context = md5::Context::new();
    let mut v_buffer = [0; 8192];

    let mut v_file = std::fs::File::open(file).context("Get path file error")?;

    loop {
        let n = io::Read::read(&mut v_file, &mut v_buffer)?;
        if n == 0 {
            break;
        }
        v_context.consume(&v_buffer[..n]);
    }

    let md5_video = v_context.compute();
    let hex_video = hex::encode(md5_video.as_ref());

    Ok((size, hex_video))
}

async fn download_file(url: Url, mut file: &NamedTempFile) -> Result<()> {
    let mut pic = reqwest::get(url)
        .map_err(|err| anyhow::anyhow!("url can't get,err: {:?}", err))
        .await?;

    loop {
        match pic.chunk().await {
            Ok(Some(chunk)) => {
                file.write_all(&chunk)?;
                file.flush()?;
            }
            Ok(None) => break,
            Err(e) => {
                return Err(anyhow!("Failed to pic: {e}"));
            }
        }
    }

    Ok(())
}
