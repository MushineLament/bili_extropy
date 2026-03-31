use std::{
    fs::{self},
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use api_req::ApiCaller as _;
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
    normalization::{IndexAudio, IndexOuput, IndexVideo},
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

    let status = db.get_active_status().await?;

    for status in status {
        let path = Path::new(&status.path);

        let file = path.join(&status.name);

        db.upsert_medias([media.clone()])
            .await
            .context("can't not upsert media in to table")?;

        download(&m, db, &media, bars.clone(), &file).await?;
    }

    Ok(())
}

pub async fn download(
    response: &crate::response::Media,
    db: &Db,
    media: &media::MediaModel,
    bars: MultiProgress,
    path: &Path,
) -> Result<()> {
    let file = db
        .get_active_status()
        .await
        .context("get active status error,when download by clone")?;

    let folders = file
        .iter()
        .map(|model| Path::new(&model.path).join(&model.name))
        .map(|path| {
            if !path.exists() {
                return fs::create_dir_all(&path).map(|_| path);
            }

            if !path.is_dir() {
                return io::Result::Err(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "The path is not a directory",
                ));
            }

            Ok(path)
        })
        .filter_map(|path| {
            if let Err(err) = &path {
                error!("get or create active status err:{:?}", err);
            }
            path.ok()
        })
        .collect::<Vec<_>>();

    let icon_file =
        NamedTempFile::new_in(path).context("can't not create icon download temp file")?;

    download_file(response.pic.clone(), &icon_file)
        .await
        .context("can't not download icon")?;

    match BiliApi::request(MediaInfoAidPayload { aid: media.aid }).await? {
        MediaInfoResp {
            data: Some(MediaInfoData { pages, .. }),
            code: 0,
            ..
        } => {
            let only1p = pages.len() == 1;
            for Page { cid, page, part } in pages {
                let filename = if only1p {
                    format!("{}-{}", media.aid, media.title)
                } else {
                    format!("{}-{}({page})-{part}", media.aid, media.title)
                };
                let DashResp {
                    data:
                        DashData {
                            dash: Dash { video, audio },
                        },
                    ..
                } = BiliApi::request(DashPayload::new(media.aid, cid).await?).await?;

                let mut video = video.into_iter();
                let mut audio = audio.into_iter();

                // 目前仅支持下载按顺序的质量且为默认值，
                // 后续将设定支持修改下载
                match (video.next(), audio.next()) {
                    (Some(v), Some(a)) => {
                        let resp_v = BiliApi::client().get(v.base_url.clone()).send().await?;
                        let resp_a = BiliApi::client().get(a.base_url.clone()).send().await?;

                        let hv2u64 =
                            |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };

                        let size = hv2u64(&resp_v.headers()[CONTENT_LENGTH])
                            + hv2u64(&resp_a.headers()[CONTENT_LENGTH]);

                        let pb = create_progress_bar(size, &bars, &part);

                        let file_v = NamedTempFile::new_in(path)
                            .context("Can't not create video temp download file in directory 1")
                            .unwrap();
                        let file_a = NamedTempFile::new_in(path)
                            .context("Can't not create audio temp download file in directory 1")
                            .unwrap();

                        let file_index = NamedTempFile::new_in(path)
                            .context("Can't not create index temp download file in directory 1")
                            .unwrap();

                        download_video_audio(
                            media.aid,
                            Some(&v),
                            Some(&a),
                            Some(&file_v),
                            Some(&file_a),
                            &pb,
                        )
                        .await
                        .expect("处理下载视频的函数发生错误 1");

                        let (v_size, v_hex) = get_size_md5(&file_v)?;
                        let (a_size, a_hex) = get_size_md5(&file_a)?;

                        let _ = upsert_to_index_temp(
                            &file_index,
                            &IndexOuput {
                                video: [IndexVideo {
                                    id: v.id.clone(),
                                    base_url: v.base_url.clone(),
                                    backup_url: v.backup_url.clone(),
                                    bandwidth: v.bandwidth,
                                    codecid: v.codecid,
                                    md5: v_hex,
                                    size: v_size,
                                    audio_id: a.id,
                                    no_rexcode: false,
                                    frame_rate: v.frame_rate.clone(),
                                    width: v.width,
                                    height: v.height,
                                    widevinePssh: "".to_string(),
                                    bilidrmUri: "".to_string(),
                                }]
                                .to_vec(),
                                audio: [IndexAudio {
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
                                    widevinePssh: "".to_string(),
                                    bilidrmUri: "".to_string(),
                                }]
                                .to_vec(),
                            },
                        );

                        file_into_folder(
                            folders
                                .iter()
                                .map(|folder| folder.join(media.aid.to_string()))
                                .map(|aid_foler| {
                                    aid_foler.join(format!("c_{}", media.cid.to_string()))
                                })
                                .map(|up_cid| up_cid.join(v.id.to_string())),
                            &[
                                ("video.m4s", &file_v),
                                ("audio.m4s", &file_a),
                                ("index.json", &file_index),
                            ],
                        );
                    }
                    (Some(v), None) => {
                        let resp_v = BiliApi::client().get(v.base_url.clone()).send().await?;
                        let hv2u64 =
                            |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };
                        let size = hv2u64(&resp_v.headers()[CONTENT_LENGTH]);

                        let pb = create_progress_bar(size, &bars, &part);

                        let file_v = NamedTempFile::new_in(path)
                            .context("Can't not create video temp download file in directory 2")?;

                        let file_index = NamedTempFile::new_in(path)
                            .context("Can't not create index temp download file in directory 2")
                            .unwrap();

                        download_video_audio(media.aid, Some(&v), None, Some(&file_v), None, &pb)
                            .await
                            .expect("处理下载视频的函数发生错误 2");

                        let (v_size, v_hex) = get_size_md5(&file_v)?;

                        let _ = upsert_to_index_temp(
                            &file_index,
                            &IndexOuput {
                                video: [IndexVideo {
                                    id: v.id.clone(),
                                    base_url: v.base_url.clone(),
                                    backup_url: v.backup_url.clone(),
                                    bandwidth: v.bandwidth,
                                    codecid: v.codecid,
                                    md5: v_hex,
                                    size: v_size,
                                    audio_id: 0,
                                    no_rexcode: false,
                                    frame_rate: v.frame_rate.clone(),
                                    width: v.width,
                                    height: v.height,
                                    widevinePssh: "".to_string(),
                                    bilidrmUri: "".to_string(),
                                }]
                                .to_vec(),
                                audio: [].to_vec(),
                            },
                        );

                        file_into_folder(
                            folders
                                .iter()
                                .map(|folder| folder.join(media.aid.to_string()))
                                .map(|aid_foler| {
                                    aid_foler.join(format!("c_{}", media.cid.to_string()))
                                })
                                .map(|up_cid| up_cid.join(v.id.to_string())),
                            &[("video.m4s", &file_v), ("index.json", &file_index)],
                        );
                    }
                    (None, Some(a)) => {
                        let resp_a = BiliApi::client().get(a.base_url.clone()).send().await?;
                        let hv2u64 =
                            |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };
                        let size = hv2u64(&resp_a.headers()[CONTENT_LENGTH]);

                        let pb = create_progress_bar(size, &bars, &part);

                        let file_a = NamedTempFile::new_in(path)
                            .context("Can't not create audio temp download file in directory 2")?;

                        let file_index = NamedTempFile::new_in(path)
                            .context("Can't not create index temp download file in directory 2")
                            .unwrap();

                        download_video_audio(media.aid, None, Some(&a), None, Some(&file_a), &pb)
                            .await
                            .expect("处理下载视频的函数发生错误 3");

                        let (a_size, a_hex) = get_size_md5(&file_a)?;

                        let _ = upsert_to_index_temp(
                            &file_index,
                            &IndexOuput {
                                video: [].to_vec(),
                                audio: [IndexAudio {
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
                                    widevinePssh: "".to_string(),
                                    bilidrmUri: "".to_string(),
                                }]
                                .to_vec(),
                            },
                        );

                        file_into_folder(
                            folders
                                .iter()
                                .map(|folder| folder.join(media.aid.to_string()))
                                .map(|aid_foler| {
                                    aid_foler.join(format!("c_{}", media.cid.to_string()))
                                })
                                .map(|up_cid| up_cid.join(a.id.to_string())),
                            &[("audio.m4s", &file_a), ("index.json", &file_index)],
                        );
                    }
                    (None, None) => return Err(anyhow!("No legal stream in {}", filename)),
                }
            }
            db.set_media_state(media.aid, MediaState::Completed).await?;
            Ok(())
        }
        MediaInfoResp {
            code,
            message: option_msg,
            ..
        } => {
            db.set_media_state(
                media.aid,
                match code {
                    -403 | 62012 | 62002 => MediaState::PermissionDenied,
                    _ => MediaState::Expired,
                },
            )
            .await?;
            Err(anyhow!(
                "Info unreachable media<{}-{}>: {}",
                media.aid,
                media.title,
                option_msg.unwrap_or_default()
            ))
        }
    }
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
            res = async { resp_v.as_mut().unwrap().chunk().await }, if !finished_v && resp_v.is_some() && file_v.is_some() => {
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

            res = async { resp_a.as_mut().unwrap().chunk().await }, if !finished_a && resp_a.is_some() && file_a.is_some() => {
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
    iter: &[(&'a str, &'a NamedTempFile)],
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
            if let Err(err) = fs::copy(metadata, folder.join(file_name)) {
                error!(
                    "file_name: {:?},path: {:?},err: {:?}",
                    folder, file_name, err
                );
            }
        }
    }
}

fn upsert_to_index_temp(file: &NamedTempFile, output: &IndexOuput) -> Result<()> {
    let writer = io::BufWriter::new(file);
    Ok(serde_json::to_writer_pretty(writer, &output).context("Can't upsert into index.json")?) // 使用 pretty 格式化输出
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
    let mut pic = reqwest::get(url).await.context("url can't get")?;

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
