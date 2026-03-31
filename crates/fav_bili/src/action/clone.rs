use std::{
    fs::{self, File},
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use api_req::{ApiCaller as _, Payload};
use avmux::{AFile, Mux as _, VFile};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::header::{CONTENT_LENGTH, HeaderValue};
use sea_orm::ColumnTrait as _;
use tempfile::NamedTempFile;
use tracing::error;

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

    let mut pic = reqwest::get(m.pic).await?;

    let mut pic_file = NamedTempFile::new_in(".")
        .context("Can't not create index temp download file in directory 1")
        .unwrap();

    loop {
        match pic.chunk().await {
            Ok(Some(chunk)) => {
                pic_file.write_all(&chunk)?;
                pic_file.flush()?;
            }
            Ok(None) => break,
            Err(e) => {
                return Err(anyhow!("Failed to pic: {e}"));
            }
        }
    }

    fs::copy(pic_file, "cover.jpg");

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

        download(db, &media, bars.clone(), &file).await?;
    }

    Ok(())
}

pub async fn download(
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

                let test = DashPayload::new(media.aid, cid).await?;
                println!("DashPayload:{:#?}", test);

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

                        // panic!("刻意终止");

                        video_audio_into_folder(
                            media,
                            folders.iter(),
                            Some(&file_v),
                            Some(&file_a),
                            v.id,
                        );

                        let (v_size, v_hex) = get_size_md5(&file_v)?;
                        let (a_size, a_hex) = get_size_md5(&file_a)?;

                        let _ = upsert_index_temp(
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

                        index_into_folder(media, folders.iter(), &file_index, v.id);
                    }
                    (Some(v), None) => {
                        let resp_v = BiliApi::client().get(v.base_url.clone()).send().await?;
                        let hv2u64 =
                            |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };
                        let size = hv2u64(&resp_v.headers()[CONTENT_LENGTH]);

                        let pb = create_progress_bar(size, &bars, &part);

                        let file_v = NamedTempFile::new_in(path)
                            .context("Can't not create video temp download file in directory 2")?;

                        download_video_audio(media.aid, Some(&v), None, Some(&file_v), None, &pb)
                            .await
                            .expect("处理下载视频的函数发生错误 2");

                        video_audio_into_folder(media, folders.iter(), Some(&file_v), None, v.id);
                    }
                    (None, Some(a)) => {
                        let resp_a = BiliApi::client().get(a.base_url.clone()).send().await?;
                        let hv2u64 =
                            |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };
                        let size = hv2u64(&resp_a.headers()[CONTENT_LENGTH]);

                        let pb = create_progress_bar(size, &bars, &part);

                        let file_a = NamedTempFile::new_in(path)
                            .context("Can't not create audio temp download file in directory 2")?;

                        download_video_audio(media.aid, None, Some(&a), None, Some(&file_a), &pb)
                            .await
                            .expect("处理下载视频的函数发生错误 3");

                        video_audio_into_folder(media, folders.iter(), None, Some(&file_a), 0);
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

fn video_audio_into_folder<'a>(
    media: &media::MediaModel,
    folders: impl Iterator<Item = &'a PathBuf>,
    file_v: Option<&NamedTempFile>,
    file_a: Option<&NamedTempFile>,
    clarity_id: i64,
) {
    for folder in folders {
        let aid_path = folder.join(media.aid.to_string());
        let cid_path = aid_path.join(format!("c_{}", media.cid.to_string()));

        // 80为清晰度，但考虑目前还没有支持清晰度的选定，暂定80
        let cid_path = cid_path.join(clarity_id.to_string());

        if !cid_path.exists()
            && let Err(err) = fs::create_dir_all(&cid_path)
        {
            error!("create dir error, path:{:?}, err:{:?},", cid_path, err);
            continue;
        }

        if !cid_path.is_dir() {
            error!("not is a directory, path:{:?}", cid_path);
        }

        let video_path = cid_path.join(format!("video.m4s"));
        let audio_path = cid_path.join(format!("audio.m4s"));

        // 后期改为支持 文件hash检测一致性
        // 如果不一致则丢出错误
        if let Some(file_v) = file_v {
            if !video_path.exists() {
                if let Err(err) = std::fs::copy(file_v.path(), &video_path) {
                    error!("Move download file error 1:{:?}", err);
                }
            } else {
                error!("video path already exist:{:?}", video_path);
            }
        }

        if let Some(file_a) = file_a {
            if !audio_path.exists() {
                if let Err(err) = std::fs::copy(file_a.path(), &audio_path) {
                    error!("Move download file error 2:{:?}", err);
                }
            } else {
                error!("audio path already exist:{:?}", video_path);
            }
        }
    }
}

fn upsert_index_temp(file: &NamedTempFile, output: &IndexOuput) -> Result<()> {
    let writer = io::BufWriter::new(file);
    Ok(serde_json::to_writer_pretty(writer, &output).context("Can't upsert into index.json")?) // 使用 pretty 格式化输出
}

fn index_into_folder<'a>(
    media: &media::MediaModel,
    folders: impl Iterator<Item = &'a PathBuf>,
    file_index: &NamedTempFile,
    clarity_id: i64,
) {
    for folder in folders {
        let aid_path = folder.join(media.aid.to_string());
        let cid_path = aid_path.join(format!("c_{}", media.cid.to_string()));

        let cid_path = cid_path.join(clarity_id.to_string());

        if !cid_path.exists()
            && let Err(err) = fs::create_dir_all(&cid_path)
        {
            error!("create dir error, path:{:?}, err:{:?},", cid_path, err);
            continue;
        }

        let index_path = cid_path.join(format!("index.json"));

        if !index_path.exists() {
            if let Err(err) = std::fs::copy(file_index.path(), &index_path) {
                error!("Move index file error 1:{:?}", err);
            }
        } else {
            error!("index path already exist:{:?}", index_path);
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
