use std::{path::Path, sync::Arc};

use anyhow::Result;
use api_req::ApiCaller as _;
use dashmap::DashSet;
use indicatif::{MultiProgress, ProgressDrawTarget};
use sea_orm::ColumnTrait as _;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    api::BiliApi,
    cookies::{add_cookie_jar, parse_cookies},
    db::db,
    entity::account,
    payload::MediaInfoAidPayload,
    response::MediaInfoSingle,
    state::AccountState,
};

pub async fn pull() -> Result<()> {
    let db = db(false).await;
    let accounts = db
        .get_accounts_filtered(account::Column::State.eq(AccountState::Active))
        .await?;

    let pulled_medias = Arc::new(DashSet::<i64>::new());

    let medias = db.all_active_pending_medias().await?;
    let bars = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

    for account in accounts {
        info!("Pulling medias with account<{}>", account.name);
        add_cookie_jar(parse_cookies(&account.cookies));
        let token = CancellationToken::new();

        let mut tasks = medias
            .iter()
            .filter(|media| !pulled_medias.contains(&media.aid))
            .map(|media| {
                let token = token.clone();
                let db = db.clone();
                let bars = bars.clone();
                let pulled_medias = pulled_medias.clone();

                async move {
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
                }
            });

        loop {
            let Some(task) = tasks.next() else {
                break;
            };
            let _task = task.await;
        }
    }
    drop(bars);
    info!("Finished pulling");
    Ok(())
}
