use bevy::{
    app::{Plugin, PostStartup, Update},
    ecs::{
        message::MessageReader,
        system::{Commands, Res, ResMut},
    },
};
use bevy_tokio_tasks::TokioTasksRuntime;
use tracing::error;

use crate::{
    components::{
        auth::handle::{ActiveAccounts, AuthLoginTask},
        initialize::DbInitailizeResource as _,
    },
    console::ConsoleTrims,
    db::Db,
};

pub struct CommmandLoginPlugin;

impl Plugin for CommmandLoginPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostStartup, ActiveAccounts::new.to_system())
            .add_systems(Update, command_login);
    }
}

pub fn command_login(
    mut commands: Commands,
    db: Res<Db>,
    mut runtimer: ResMut<TokioTasksRuntime>,
    mut console_message: MessageReader<ConsoleTrims>,
) {
    for message in console_message.read() {
        let db = db.clone();
        let (args, _argv) = argmap::parse(message.0.iter());

        if !args.get(1).is_some_and(|list| list.eq("auth")) {
            continue;
        }

        match args.get(2).map(String::as_str) {
            Some("login") => {
                commands.spawn(AuthLoginTask::new(db, runtimer.as_mut()));
            }
            Some(unkown) => {
                error!("not has this command: {:?}", unkown);
            }
            None => {
                // 输出help
            }
        }
    }
}

// pub async fn usecookies(cookies: String) -> Result<()> {
//     let db = db(true).await;
//     add_cookie_jar(parse_cookies(&cookies));
//     let cookies = current_cookies()?;
//     let WbiResp {
//         data: WbiData { mid, uname, .. },
//     } = BiliApi::request(WbiPayload).await?;
//     db.upsert_account(account::AccountModel {
//         account_id: mid,
//         name: uname.to_owned(),
//         cookies,
//         state: AccountState::Active.to_string(),
//     })
//     .await?;
//     println!("Hello😊, {uname}.");
//     Ok(())
// }

// pub async fn logout(account_id: i64) -> Result<()> {
//     let db = db(false).await;
//     let account = db.get_account(account_id).await?;
//     logout_account(account_id, account.cookies).await?;
//     info!("Logout successfully.");
//     db.delete_account(account_id).await?;
//     println!("Goodbye👋, {}", account.name);
//     Ok(())
// }

// pub async fn logout_all() -> Result<()> {
//     let db = db(false).await;
//     let accounts = db.all_accounts().await?;
//     let mut tasks = futures::stream::iter(accounts)
//         .map(|account| async move {
//             logout_account(account.account_id, account.cookies).await?;
//             info!("Logout successfully.");
//             db.delete_account(account.account_id).await?;
//             println!("Goodbye👋, {}", account.name);
//             Ok::<_, anyhow::Error>(())
//         })
//         .buffer_unordered(8);
//     while let Some(res) = tasks.next().await {
//         if let Err(e) = res {
//             error!("{}", e);
//         }
//     }
//     Ok(())
// }

// async fn logout_account(account_id: i64, cookies: String) -> Result<()> {
//     let cookies = parse_cookies(&cookies).collect::<Vec<_>>();
//     let bili_jct = cookies
//         .iter()
//         .find(|c| c.name() == "bili_jct")
//         .map(|c| c.value().to_owned())
//         .context(format!(
//             "No bili_jct in cookies of account_id<{account_id}>."
//         ))?;
//     add_cookie_jar(cookies.into_iter());
//     let LogoutResp { code, message } =
//         AuthApi::request(LogoutPayload { biliCSRF: bili_jct }).await?;
//     match code {
//         0 => Ok(()),
//         _ => Err(anyhow!("Failed to logout: {}", message.unwrap_or_default())),
//     }
// }

// pub async fn check(account_id: i64) -> Result<()> {
//     let db = db(false).await;
//     let account = db.get_account(account_id).await?;
//     check_account(account).await?;
//     Ok(())
// }

// pub async fn check_all() -> Result<()> {
//     let db = db(false).await;
//     let accounts = db.all_accounts().await?;
//     for account in accounts {
//         check_account(account).await?;
//     }
//     Ok(())
// }

// async fn check_account(account: account::AccountModel) -> Result<()> {
//     add_cookie_jar(parse_cookies(&account.cookies));
//     match BiliApi::request(WbiPayload).await {
//         Ok(WbiResp {
//             data: WbiData { mid, .. },
//         }) => {
//             if mid == account.account_id {
//                 info!("Check passed. Hello😊, {}.", account.name);
//             } else {
//                 error!(
//                     "Bilibili returned unmatched user id account<{}>",
//                     account.name
//                 )
//             }
//         }
//         Err(ApiErr::UnDeserializeable(_)) => error!(
//             "Bilibili returned unexpected json, cookies expired: account<{}>",
//             account.name
//         ),
//         Err(e) => return Err(e.into()),
//     }
//     Ok(())
// }
