use std::{
    fs::{self, File},
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use api_req::ApiCaller as _;
use avmux::{AFile, Mux as _, VFile};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use reqwest::header::{CONTENT_LENGTH, HeaderValue};
use sea_orm::ColumnTrait as _;
use tempfile::NamedTempFile;

use crate::{
    api::BiliApi,
    cookies::{add_cookie_jar, parse_cookies},
    db::db,
    entity::{account, media},
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
        id: m.id,
        bv_id: m.bv_id.to_owned(),
        title: m.title.to_owned(),
        r#type: m.r#type.to_string(),
        state: MediaState::Pending.to_string(),
    };

    let collection = db.get_active_status().await?;

    let path = Path::new(&collection.path);

    let file = path.join(&collection.name);

    if !file.exists() {
        fs::create_dir(&file)?;
    }

    download(&media, bars, &file).await?;

    Ok(())
}

pub async fn download(media: &media::MediaModel, bars: MultiProgress, path: &Path) -> Result<()> {
    match BiliApi::request(MediaInfoAidPayload { aid: media.id }).await? {
        MediaInfoResp {
            data: Some(MediaInfoData { pages, .. }),
            code: 0,
            ..
        } => {
            let only1p = pages.len() == 1;
            for Page { cid, page, part } in pages {
                let filename = if only1p {
                    format!("{}-{}", media.id, media.title)
                } else {
                    format!("{}-{}({page})-{part}", media.id, media.title)
                };
                let DashResp {
                    data:
                        DashData {
                            dash: Dash { video, audio },
                        },
                } = BiliApi::request(DashPayload::new(media.id, cid).await?).await?;

                let mut video = video.into_iter();
                let mut audio = audio.into_iter();
                loop {
                    match (video.next(), audio.next()) {
                        (Some(v), Some(a)) => {
                            let mut resp_v = BiliApi::client().get(v.base_url).send().await?;
                            let mut resp_a = BiliApi::client().get(a.base_url).send().await?;
                            let hv2u64 =
                                |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };
                            let size = hv2u64(&resp_v.headers()[CONTENT_LENGTH])
                                + hv2u64(&resp_a.headers()[CONTENT_LENGTH]);
                            let pb = ProgressBar::new(size);
                            bars.add(pb.clone());
                            pb.set_message(head(&part, 10));
                            pb.set_style(
                                ProgressStyle::with_template(BAR_TEMPLATE)
                                    .unwrap()
                                    .progress_chars("#>-"),
                            );
                            let mut file_v = NamedTempFile::new()?;
                            let mut file_a = NamedTempFile::new()?;
                            let (mut finished_v, mut finished_a) = (false, false);
                            loop {
                                tokio::select! {
                                    res = resp_v.chunk(), if !finished_v => {
                                        match res {
                                            Ok(Some(chunk)) => {
                                                file_v.write_all(&chunk)?;
                                                file_v.flush()?;
                                                pb.inc(chunk.len() as u64);
                                            }
                                            Ok(None) => finished_v = true,
                                            Err(e) => return Err(anyhow!(
                                                "Failed to download video {filename}: {e}"
                                            ))
                                        }
                                    }
                                    res = resp_a.chunk(), if !finished_a => {
                                        match res {
                                            Ok(Some(chunk)) => {
                                                file_a.write_all(&chunk)?;
                                                file_a.flush()?;
                                                pb.inc(chunk.len() as u64);
                                            }
                                            Ok(None) => finished_a = true,
                                            Err(e) => return Err(anyhow!(
                                                "Failed to download audio {filename}: {e}"
                                            ))
                                        }
                                    }
                                    else => break,
                                }
                            }
                            pb.finish();
                            let title = format!(
                                "{filename}.mp4",
                                filename = sanitize_filename::sanitize(&filename)
                            );

                            let output_path = path.join(title);

                            let output_path = output_path
                                .to_str()
                                .context("get download folder path err")?;

                            if (
                                VFile::new(file_v.path().to_string_lossy()),
                                AFile::new(file_a.path().to_string_lossy()),
                            )
                                .simple_mux(VFile::new(&output_path))
                                .is_err()
                            {
                                std::fs::remove_file(output_path).ok();
                                continue;
                            }
                        }
                        (Some(v), None) => {
                            let mut resp_v = BiliApi::client().get(v.base_url).send().await?;
                            let hv2u64 =
                                |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };
                            let size = hv2u64(&resp_v.headers()[CONTENT_LENGTH]);
                            let pb = ProgressBar::new(size);
                            bars.add(pb.clone());
                            pb.set_message(head(part, 10));
                            pb.set_style(
                                ProgressStyle::with_template(BAR_TEMPLATE)
                                    .unwrap()
                                    .progress_chars("#>-"),
                            );
                            let mut file_v = NamedTempFile::new()?;
                            loop {
                                match resp_v.chunk().await {
                                    Ok(Some(chunk)) => {
                                        file_v.write_all(&chunk)?;
                                        file_v.flush()?;
                                        pb.inc(chunk.len() as u64);
                                    }
                                    Ok(None) => break,
                                    Err(e) => {
                                        return Err(anyhow!(
                                            "Failed to download video {filename}: {e}"
                                        ));
                                    }
                                }
                            }
                            pb.finish();
                            let title = format!(
                                "{filename}.mp4",
                                filename = sanitize_filename::sanitize(&filename)
                            );

                            tokio::fs::rename(file_v.path(), format!("./{title}")).await?;
                        }
                        (None, Some(a)) => {
                            let mut resp_a = BiliApi::client().get(a.base_url).send().await?;
                            let hv2u64 =
                                |hv: &HeaderValue| -> u64 { hv.to_str().unwrap().parse().unwrap() };
                            let size = hv2u64(&resp_a.headers()[CONTENT_LENGTH]);
                            let pb = ProgressBar::new(size);
                            bars.add(pb.clone());
                            pb.set_message(head(part, 10));
                            pb.set_style(
                                ProgressStyle::with_template(BAR_TEMPLATE)
                                    .unwrap()
                                    .progress_chars("#>-"),
                            );
                            let mut file_a = NamedTempFile::new()?;
                            loop {
                                match resp_a.chunk().await {
                                    Ok(Some(chunk)) => {
                                        file_a.write_all(&chunk)?;
                                        file_a.flush()?;
                                        pb.inc(chunk.len() as u64);
                                    }
                                    Ok(None) => break,
                                    Err(e) => {
                                        return Err(anyhow!(
                                            "Failed to download audio {filename}: {e}"
                                        ));
                                    }
                                }
                            }
                            pb.finish();
                            let title = format!(
                                "{filename}.mp3",
                                filename = sanitize_filename::sanitize(&filename)
                            );
                            tokio::fs::rename(file_a.path(), format!("./{title}")).await?;
                        }
                        (None, None) => return Err(anyhow!("No legal stream in {}", filename)),
                    }
                    break;
                }
            }
            Ok(())
        }
        MediaInfoResp {
            message: option_msg,
            ..
        } => Err(anyhow!(
            "Info unreachable media<{}-{}>: {}",
            media.id,
            media.title,
            option_msg.unwrap_or_default()
        )),
    }
}
