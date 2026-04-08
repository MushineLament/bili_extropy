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

    let medias = Arc::new(medias);

    let bars = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

    let token = CancellationToken::new();

    let token2 = token.clone();
    let tasks = accounts
        .into_iter()
        .map(move|account| {
            info!("Pulling medias with account<{}>", account.name);
            add_cookie_jar(parse_cookies(&account.cookies));
            token2.clone()
        })
        .map(   move|token| {
            let token = token;
            let medias = medias.clone();
            let db = db.clone();
            let bars = bars.clone();
            let pulled_medias = pulled_medias.clone();
            let range = 0..medias.len();
            range.clone().into_iter().map(move |id| {
                (
                    medias.clone(),
                    id,
                    token.clone(),
                    db.clone(),
                    bars.clone(),
                    pulled_medias.clone(),
                )
            })
        })
        .flatten()
        .map(|(medias, id, token, db, bars, pulled_medias)| async move {
            let m = match BiliApi::request(MediaInfoAidPayload {
                aid: medias[id].aid,
            })
            .await
            {
                Ok(MediaInfoSingle {
                    code: _,
                    data: Some(data),
                    message: _,
                }) => data,
                err => {
                    error!("Info unreachable : {:?}", err);
                    return error!(
                        "pull madie error,title: {:?} ,cid: {:?}",
                        medias[id].title, medias[id].cid
                    );
                }
            };
            tokio::select! {
                res = crate::action::clone::download(&m, &db,&medias[id], bars,&Path::new(crate::action::TEMP_DOWNLOAD_FOLDER)), if !token.is_cancelled() => match res {
                    Ok(_) => { pulled_medias.insert(medias[id].aid); }
                    Err(e) => error!("aid: {:?},bvid: {:?},download video error,{}",medias[id].aid,medias[id].bv_id, e),
                },
                _ = token.cancelled() => {},
            }
        });

    let mut tasks = futures::stream::iter(tasks).buffer_unordered(LIMIT_PAR_DOWNLOAD);

    let tasks = async move {
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
        info!("All tasks is finish");
    };

    let join = tokio::spawn(tasks);

    // 让后台任务自己运行，主函数继续或等待退出信号
    // tokio::signal::ctrl_c().await?;

    join.await?;

    info!("Finished pulling");
    Ok(())
}
