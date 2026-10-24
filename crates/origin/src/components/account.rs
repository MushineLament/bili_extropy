use std::time::Duration;

use api_req::{ApiCaller, error::ApiErr};

use migration::OnConflict;
use qrcode::{QrCode, render::unicode, types::QrError};
use sea_orm::{DbErr, EntityTrait, IntoActiveModel as _};
use tracing::{error, info};

use crate::{
    api::{AuthApi, BiliApi},
    components::account::LoginError::QrErr,
    cookies::current_cookies,
    db::Db,
    entity::{
        account::{self, AccountModel, QrData, QrPollData, QrPollResp, QrResp},
        upper::{QrPayload, QrPollPayload},
    },
    payload::WbiPayload,
    state::AccountState,
    wbi::{WbiData, WbiResp},
};

#[derive(Debug)]
pub enum LoginError {
    ApiErr(ApiErr),
    QrErr(QrError),
    DbErr(DbErr),
}

/// login account task, will block the current thread until login success or failed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveAccount(pub AccountModel);

impl ActiveAccount {
    pub async fn login(db: &Db) -> Result<Self, LoginError> {
        let QrResp {
            data: QrData { url, qrcode_key },
        } = AuthApi::request(QrPayload)
            .await
            .map_err(|e| LoginError::ApiErr(e))?;

        let code = QrCode::new(url.as_ref()).map_err(|e| QrErr(e))?;

        let image = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        println!("{}", image);

        loop {
            let Ok(QrPollResp {
                data: QrPollData { code, message },
            }) = AuthApi::request(QrPollPayload {
                qrcode_key: qrcode_key.clone(),
            })
            .await
            else {
                continue;
            };

            match code {
                0 => {
                    info!("Login successfully.");
                    break;
                }
                86101 | 86090 => {
                    // 用户还没扫码/没确认，睡一会再来
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
                _ => {
                    error!("Login error: {}", message);
                    break;
                }
            }
        }

        let cookies = current_cookies();

        let WbiResp {
            data: WbiData { mid, uname, .. },
        } = BiliApi::request(WbiPayload)
            .await
            .map_err(|e| LoginError::ApiErr(e))?;

        let model = account::AccountModel {
            account_id: mid,
            name: uname.to_owned(),
            cookies: cookies.to_owned(),
            state: AccountState::Active.to_string(),
        };

        // insert to db
        let _insert = account::AccountEntity::insert(model.clone().into_active_model())
            .on_conflict(
                OnConflict::column(account::Column::AccountId)
                    .update_columns([account::Column::Name, account::Column::Cookies])
                    .to_owned(),
            )
            .exec(&db.0)
            .await
            .map_err(|e| LoginError::DbErr(e))?;

        println!("Hello😊, {uname}.");

        Ok(Self(model))
    }

    // pub fn id(&self) -> Option<i64> {
    //     self.0.as_ref().map(|account| account.account_id).ok()
    // }

    // pub fn model(&self) -> Option<&AccountModel> {
    //     self.0.as_ref().ok()
    // }

    // pub fn cookies(&self) -> Option<&str> {
    //     self.0.as_ref().ok().map(|account| account.cookies.as_str())
    // }
}

// #[derive(Debug, Component, Deref, DerefMut)]
// pub struct LoadAccountCollectionsTask(
//     pub ECSHandleResult<Vec<AccountCollectionModel>, anyhow::Error>,
// );

// impl LoadAccountCollectionsTask {
//     pub fn new(db: Db, runtimer: &mut TokioTasksRuntime) -> Self {
//         let task = async move {
//             let medias = AccountCollectionEntity::find().all(&db.db).await?;
//             Ok(medias)
//         };
//         let handle = runtimer.spawn_background_task(|_ctx| task);
//         Self(ECSHandleResult::new(handle))
//     }
// }
