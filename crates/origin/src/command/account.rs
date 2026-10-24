use std::sync::LazyLock;

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use sea_orm::EntityTrait;
use tracing::error;

use crate::{
    RUNTIMER,
    components::account::ActiveAccount,
    db::DB,
    entity::account::{self},
};

pub fn command_auth() -> Command {
    let _ = (
        Command::new("activate")
            .about("Activate obj [alias: active, a]")
            .arg_required_else_help(true)
            .aliases(["active", "a"])
            .subcommands([
                Command::new("account")
                    .about("Activate accounts [alias: a]")
                    .arg_required_else_help(true)
                    .aliases(["a"])
                    .args([
                        Arg::new("all")
                            .help("Activate all authorized accounts")
                            .long("all")
                            .short('a')
                            .action(ArgAction::SetTrue)
                            .conflicts_with("account_id"),
                        Arg::new("account_id")
                            .help("The account to activate")
                            .value_parser(value_parser!(i64))
                            .action(ArgAction::Append),
                    ]),
                Command::new("set")
                    .about("Activate sets [alias: list, collection, s, l, c]")
                    .arg_required_else_help(true)
                    .aliases(["list", "collection", "s", "l", "c"])
                    .args([
                        Arg::new("all")
                            .help("Activate all sets")
                            .long("all")
                            .short('a')
                            .action(ArgAction::SetTrue)
                            .conflicts_with("set_id"),
                        Arg::new("set_id")
                            .help("The set to activate")
                            .value_parser(value_parser!(i64))
                            .action(ArgAction::Append),
                    ]),
                Command::new("up")
                    .about("Activate ups [alias: u]")
                    .arg_required_else_help(true)
                    .aliases(["u"])
                    .args([
                        Arg::new("all")
                            .help("Activate all ups")
                            .long("all")
                            .short('a')
                            .action(ArgAction::SetTrue)
                            .conflicts_with("up_id"),
                        Arg::new("up_id")
                            .help("The up to activate")
                            .value_parser(value_parser!(i64))
                            .action(ArgAction::Append),
                    ]),
            ]),
        Command::new("deactivate")
            .about("Deactivate obj [alias: d]")
            .aliases(["d"])
            .arg_required_else_help(true)
            .subcommands([
                Command::new("account")
                    .about("Deactivate accounts [alias: a]")
                    .arg_required_else_help(true)
                    .aliases(["a"])
                    .args([
                        Arg::new("all")
                            .help("Dectivate all authorized accounts")
                            .long("all")
                            .short('a')
                            .action(ArgAction::SetTrue)
                            .conflicts_with("account_id"),
                        Arg::new("account_id")
                            .help("The account to deactivate")
                            .value_parser(value_parser!(i64))
                            .action(ArgAction::Append),
                    ]),
                Command::new("set")
                    .about("Deactivate sets [alias: list, collection, s, l, c]")
                    .arg_required_else_help(true)
                    .aliases(["list", "collection", "s", "l", "c"])
                    .args([
                        Arg::new("all")
                            .help("Deactivate all sets")
                            .long("all")
                            .short('a')
                            .action(ArgAction::SetTrue)
                            .conflicts_with("set_id"),
                        Arg::new("set_id")
                            .help("The set to deactivate")
                            .value_parser(value_parser!(i64))
                            .action(ArgAction::Append),
                    ]),
                Command::new("up")
                    .about("Deactivate ups [alias: u]")
                    .arg_required_else_help(true)
                    .aliases(["u"])
                    .args([
                        Arg::new("all")
                            .help("Deactivate all ups")
                            .long("all")
                            .short('a')
                            .action(ArgAction::SetTrue)
                            .conflicts_with("up_id"),
                        Arg::new("up_id")
                            .help("The up to deactivate")
                            .value_parser(value_parser!(i64))
                            .action(ArgAction::Append),
                    ]),
            ]),
    );
    Command::new("auth")
                        .about("Auth account")
                        .arg_required_else_help(true)
                        .subcommands([
                            Command::new("login").about("Login with QR code"),
                            Command::new("usecookies")
                                .about("Add accounts with user-provided cookies (recommended)")
                                .arg_required_else_help(true)
                                .args([
                                    Arg::new("cookies")
                                        .help(
"Cookies at least including SESSDATA; For logout, plus DedeUserID, bili_jct; For liking medias, please copy directly from browser"
                                        ).action(ArgAction::Append)
                                ]),
                            Command::new("logout")
                                .about("Logout accounts")
                                .arg_required_else_help(true)
                                .args([
                                    Arg::new("all")
                                        .help("Logout all authorized accounts")
                                        .long("all")
                                        .short('a')
                                        .action(ArgAction::SetTrue)
                                        .conflicts_with("account_id"),
                                    Arg::new("account_id")
                                        .help("The account to logout")
                                        .value_parser(value_parser!(i64))
                                        .action(ArgAction::Append),
                                ]),
                            Command::new("check")
                                .about("Check accounts cookies usability")
                                .arg_required_else_help(true)
                                .args([
                                    Arg::new("all")
                                        .help("Check all accounts cookies usability")
                                        .long("all")
                                        .short('a')
                                        .action(ArgAction::SetTrue)
                                        .conflicts_with("account_id"),
                                    Arg::new("account_id")
                                        .help("The account to check cookies usability")
                                        .value_parser(value_parser!(i64))
                                        .action(ArgAction::Append),
                                ]),
                        ])
}

pub fn execute_auth(arg: &ArgMatches) {
    match arg.subcommand() {
        Some(("login", _)) => {
            if let Err(err) = RUNTIMER.block_on(ActiveAccount::login(&DB)) {
                error!("login error: {:?}", err);
            }
        }
        _ => unreachable!(),
    }
}

pub const ACTIVE_ACCOUNT: LazyLock<ActiveAccount> = LazyLock::new(|| {
    RUNTIMER
        .block_on(account::AccountEntity::find().one(&DB.0))
        .expect("Loading account data error: ")
        .map(|model| ActiveAccount(model))
        .or_else(|| RUNTIMER.block_on(ActiveAccount::login(&DB)).ok())
        .expect("Not has any account")
});

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
