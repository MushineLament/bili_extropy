use std::{path::Path, sync::Arc};

use anyhow::Result;
use api_req::ApiCaller as _;
use dashmap::DashSet;
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressDrawTarget};
use sea_orm::ColumnTrait as _;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    api::BiliApi,
    cookies::{add_cookie_jar, parse_cookies},
    db::db,
    entity::account,
    payload::MediaInfoAidPayload,
    response::MediaInfoSingle,
    state::AccountState,
};

pub const LIMIT_PAR_DOWNLOAD: usize = 3;

pub async fn pull() -> Result<()> {
    let db = db(false).await;
    let accounts = db
        .get_accounts_filtered(account::Column::State.eq(AccountState::Active))
        .await?;

    let pulled_medias = Arc::new(DashSet::<i64>::new());

    let medias = db.all_active_pending_medias().await?;
    let bars = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

    let mut tasks = accounts.into_iter().map(|account| {
        info!("Pulling medias with account<{}>", account.name);
        add_cookie_jar(parse_cookies(&account.cookies));
        CancellationToken::new()
    }).map(|token|{
        let token2 = token.clone();
        (medias
            .iter()
            .filter(|media| !pulled_medias.contains(&media.aid))
            .map(move |media|(media,token.clone()))
            .map(|(media,token)| {
                let db = db.clone();
                let bars = bars.clone();
                let pulled_medias = pulled_medias.clone();

                let task = async move {
                    let m = match BiliApi::request(MediaInfoAidPayload { aid: media.aid }).await {
                        Ok(MediaInfoSingle {
                            code: _,
                            data: Some(data),
                            message: _,
                        }) => data,
                        err => {
                            error!("Info unreachable : {:?}", err);
                            return error!(
                                "pull madie error,title: {:?} ,cid: {:?}",
                                media.title, media.cid
                            );
                        }
                    };
                    tokio::select! {
                        res = crate::action::clone::download(&m, &db,media, bars,&Path::new(crate::action::TEMP_DOWNLOAD_FOLDER)), if !token.is_cancelled() => match res {
                            Ok(_) => { pulled_medias.insert(media.aid); }
                            Err(e) => error!("aid: {:?},bvid: {:?},download video error,{}",media.aid,media.bv_id, e),
                        },
                        _ = token.cancelled() => {},
                    }
                };
                task
            }),token2)
    });

    loop {
        let Some((tasks, token)) = tasks.next() else {
            break;
        };

        let mut tasks = futures::stream::iter(tasks).buffer_unordered(LIMIT_PAR_DOWNLOAD);

        loop {
            tokio::select! {
                res = tasks.next() => {
                    if res.is_none() {
                        break;
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    token.cancel();
                    warn!("Received Ctrl-C");
                    break;
                }
            }
        }
    }

    drop(bars);
    info!("Finished pulling");
    Ok(())
}
