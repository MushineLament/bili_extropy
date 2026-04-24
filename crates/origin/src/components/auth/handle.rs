use std::time::Duration;

use api_req::ApiCaller;
use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::{Deref, DerefMut},
};
use bevy_tokio_tasks::TokioTasksRuntime;
use migration::OnConflict;
use qrcode::{QrCode, render::unicode};
use sea_orm::{ColumnTrait as _, EntityTrait, IntoActiveModel as _, QueryFilter as _};
use tracing::{error, info};

use crate::{
    api::{AuthApi, BiliApi},
    components::handle::ECSHandleResult,
    cookies::current_cookies,
    db::Db,
    entity::{
        account::{self, AccountModel, QrData, QrPollData, QrPollResp, QrResp},
        up::{QrPayload, QrPollPayload},
    },
    payload::WbiPayload,
    state::AccountState,
    wbi::{WbiData, WbiResp},
};

#[derive(Debug, Component, Deref, DerefMut)]
pub struct AuthLoginTask(pub ECSHandleResult<AccountModel, anyhow::Error>);

impl AuthLoginTask {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let QrResp {
                data: QrData { url, qrcode_key },
            } = AuthApi::request(QrPayload).await?;

            let code = QrCode::new(url.as_ref())?;

            let image = code
                .render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Light)
                .light_color(unicode::Dense1x2::Dark)
                .build();

            println!("{}", image);

            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;
                let QrPollResp {
                    data: QrPollData { code, message },
                } = AuthApi::request(QrPollPayload {
                    qrcode_key: qrcode_key.clone(),
                })
                .await?;
                match code {
                    0 => {
                        info!("Login successfully.");
                        break;
                    }
                    86101 | 86090 => {}
                    _ => {
                        error!("{}", message);
                        return Err(anyhow::anyhow!("{}", message));
                    }
                }
            }

            let cookies = current_cookies()?;

            let WbiResp {
                data: WbiData { mid, uname, .. },
            } = BiliApi::request(WbiPayload).await?;

            let model = account::AccountModel {
                account_id: mid,
                name: uname.to_owned(),
                cookies,
                state: AccountState::Active.to_string(),
            };

            // insert to db
            account::AccountEntity::insert(model.clone().into_active_model())
                .on_conflict(
                    OnConflict::column(account::Column::AccountId)
                        .update_columns([account::Column::Name, account::Column::Cookies])
                        .to_owned(),
                )
                .exec_without_returning(&db.db)
                .await?;

            println!("Hello😊, {uname}.");

            Ok(model)
        };

        let task = runtimer.spawn_background_task(move |_ctx| task);

        let task = ECSHandleResult::new(task);

        Self(task)
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct ActiveAccounts(pub ECSHandleResult<Vec<AccountModel>, anyhow::Error>);

impl ActiveAccounts {
    pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
        let task = async move {
            let accounts = account::AccountEntity::find()
                .filter(account::Column::State.eq(AccountState::Active))
                .all(&db.db)
                .await?;

            Ok(accounts)
        };

        let task = runtimer.spawn_background_task(move |_ctx| task);

        let task = ECSHandleResult::new(task);

        Self(task)
    }
}
