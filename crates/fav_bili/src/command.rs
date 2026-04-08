use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command, command, value_parser};
use clap_complete::Shell;
use rustyline::{DefaultEditor, error::ReadlineError};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::{action::*, version::VERSION};

#[derive(Debug, Default)]
pub struct FavCommand(pub Command);

impl FavCommand {
    pub fn new() -> Self {
        Self(
            command!()
                .arg_required_else_help(true)
                .version(VERSION)
                .subcommands([
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
                        ]),
                    Command::new("list")
                        .about("List accounts/sets/ups/medias [alias: ls, l]")
                        .arg_required_else_help(true)
                        .aliases(["ls", "l"])
                        .subcommands([
                            Command::new("account")
                                .about("List accounts [alias: a]")
                                .aliases(["a"]),
                            Command::new("set")
                                .about("List sets [alias: list, collection, s, l, c]")
                                .aliases(["list", "collection", "s", "l", "c"]),
                            Command::new("up")
                                .about("List ups [alias: upper, u]")
                                .aliases(["upper", "u"]),
                            Command::new("media")
                                .about("List medias [alias: video bv, m, v]")
                                .aliases(["video", "bv", "m", "v"]),
                            Command::new("status")
                                .about("List status [alias: id, i]")
                                .aliases(["id", "i"]),
                        ]),
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
                    Command::new("status")
                        .about("download media into folder [alias: s]")
                        .aliases(["s"])
                        .subcommand(
                        Command::new("set")
                            .about("Set download folder")
                            .args([
                                Arg::new("name")
                                    .help("Name of the folder")
                                    .required(true),
                                Arg::new("path")
                                    .help("Path to the folder")
                                    .required(true),
                                Arg::new("switch")
                                    .help("switch download into this path folder")
                                    .long("switch")
                                    .short('a')
                                    .action(ArgAction::SetTrue),
                            ])
                    ),
                    Command::new("fetch")
                        .about("Fetch metadata of following ups, fav sets, medias, ups [alias: f]")
                        .aliases(["f"])
                        .args([Arg::new("prune")
                            .long("prune")
                            .short('p')
                            .help("Prune the objs: remove unfaved sets, unfollowed ups and medias not belonging to active set or up")
                            .action(ArgAction::SetTrue)]),
                    Command::new("pull")
                        .about("Pull fetched medias [alias: p]")
                        .aliases(["p"]),
                    Command::new("resp")
                        .about("cli -> resp [alias: r]")
                        .aliases(["r"]),
                    Command::new("clone")
                        .about("download single medias [alias: c]")
                        .aliases(["c"]).args([
                            Arg::new("bv_id")
                                .help("The set to deactivate")
                                .value_parser(value_parser!(String))
                                .action(ArgAction::Append),
                        ]),
                    Command::new("like")
                        .about("Like medias")
                        .arg_required_else_help(true)
                        .args([
                            Arg::new("avids")
                                .help("The avids to like")
                                .value_parser(value_parser!(i64))
                                .action(ArgAction::Append)
                        ]),
                    Command::new("completion")
                        .about("Generate completion script")
                        .arg_required_else_help(true)
                        .args([Arg::new("shell")
                            .help("The shell to generate completion script for")
                            .value_parser(value_parser!(Shell))]),
                ])
                .args([Arg::new("verbose")
                    .help("Show debug messages")
                    .long("verbose")
                    .short('v')
                    .action(ArgAction::SetTrue)]),
        )
    }

    pub fn initialize_log(&mut self) {
        let matches = self.0.get_matches_mut();
        match matches.get_flag("verbose") {
            true => {
                let filter =
                    EnvFilter::from_default_env().add_directive("fav=debug".parse().unwrap());
                tracing_subscriber::fmt()
                    .with_env_filter(filter)
                    .with_thread_ids(true)
                    .with_line_number(true)
                    .init();
            }
            false => {
                let filter =
                    EnvFilter::from_default_env().add_directive("fav=info".parse().unwrap());
                tracing_subscriber::fmt()
                    .with_target(false)
                    .with_env_filter(filter)
                    .without_time()
                    .init();
            }
        }
    }

    /// Parse the commands and args, return the Event to trigger.
    pub async fn run_cli(&mut self) -> Result<()> {
        self.initialize_log();

        let matches = self.0.get_matches_mut();

        if let Some(("resp", _)) = matches.subcommand() {
            self.run_resp().await?;
            return Ok(());
        }

        self.exec_matches(matches).await?;

        Ok(())
    }

    pub async fn run_resp(&mut self) -> Result<()> {
        // 初始化 rustyline 编辑器
        let mut rl = DefaultEditor::new()?;
        if rl.load_history(".fav_history").is_err() {
            // 无历史文件则忽略
        }

        println!("Interactive mode. Type 'exit' or 'quit' to quit.\n");

        loop {
            let readline = rl.readline("bili_extropy> ");
            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    // 处理退出命令
                    if matches!(line, "exit" | "quit" | "q") {
                        break;
                    }
                    // 解析并执行命令
                    if let Err(e) = self.execute_line(line).await {
                        eprintln!("Error: {}", e);
                    }
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                    // Ctrl+C 或 Ctrl+D 退出
                    break;
                }
                Err(err) => {
                    eprintln!("Input error: {}", err);
                    break;
                }
            }
        }

        Ok(())
    }

    /// 解析一行输入并执行对应的异步动作
    async fn execute_line(&mut self, line: &str) -> Result<()> {
        // 将输入拆分为类似 argv 的列表

        if line.to_lowercase() == "mushinelament" {
            info!("!The world is Unworld!");
            return Ok(());
        }

        let mut process_name = vec!["bili_extropy".to_string()];

        process_name.extend(shell_words::split(line).context("Failed to parse command")?);

        if process_name.len() <= 1 {
            return Ok(());
        }

        let cmd = self.0.clone();
        let matches = cmd.try_get_matches_from(process_name)?;

        // 执行子命令（完全复用原有 match 逻辑）
        self.exec_matches(matches).await
    }

    pub async fn exec_matches(&mut self, matches: clap::ArgMatches) -> Result<()> {
        match matches.subcommand() {
            Some(("completion", sub_matches)) => {
                let bin_name = std::env::current_exe()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let shell = *sub_matches.get_one::<Shell>("shell").unwrap();
                clap_complete::generate(shell, &mut self.0, bin_name, &mut std::io::stdout());
            }
            sub_cmd => {
                match sub_cmd {
                    Some(("auth", sub_matches)) => match sub_matches.subcommand() {
                        Some(("login", _)) => login().await?,
                        Some(("usecookies", sub_matches)) => {
                            for cookies in sub_matches.get_many::<String>("cookies").unwrap()
                            // arg_required_else_help has been set to true
                            {
                                usecookies(cookies.to_owned()).await?;
                            }
                        }
                        Some(("logout", sub_matches)) if sub_matches.get_flag("all") => {
                            logout_all().await?;
                        }
                        Some(("logout", sub_matches)) => {
                            for account_id in sub_matches.get_many::<i64>("account_id").unwrap() {
                                logout(*account_id).await?;
                            }
                        }
                        Some(("check", sub_matches)) if sub_matches.get_flag("all") => {
                            check_all().await?;
                        }
                        Some(("check", sub_matches)) => {
                            for account_id in sub_matches.get_many::<i64>("account_id").unwrap() {
                                check(*account_id).await?;
                            }
                        }
                        _ => unreachable!(),
                    },
                    Some(("list", sub_matches)) => match sub_matches.subcommand() {
                        Some(("account", _)) => list_accounts().await?,
                        Some(("set", _)) => list_sets().await?,
                        Some(("up", _)) => list_ups().await?,
                        Some(("media", _)) => list_medias().await?,
                        Some(("status", _)) => list_status().await?,
                        _ => unreachable!(),
                    },
                    Some(("activate", sub_matches)) => match sub_matches.subcommand() {
                        Some(("account", sub_matches)) => match sub_matches.get_flag("all") {
                            true => activate_all_accounts().await?,
                            false => {
                                for account_id in sub_matches.get_many::<i64>("account_id").unwrap()
                                // arg_required_else_help has been set to true
                                {
                                    activate_account(*account_id).await?;
                                }
                            }
                        },
                        Some(("set", sub_matches)) => match sub_matches.get_flag("all") {
                            true => activate_all_sets().await?,
                            false => {
                                for set_id in sub_matches.get_many::<i64>("set_id").unwrap()
                                // arg_required_else_help has been set to true
                                {
                                    activate_set(*set_id).await?;
                                }
                            }
                        },
                        Some(("up", sub_matches)) => match sub_matches.get_flag("all") {
                            true => activate_all_ups().await?,
                            false => {
                                for up_id in sub_matches.get_many::<i64>("up_id").unwrap()
                                // arg_required_else_help has been up to true
                                {
                                    activate_up(*up_id).await?;
                                }
                            }
                        },
                        _ => unreachable!(),
                    },
                    Some(("deactivate", sub_matches)) => match sub_matches.subcommand() {
                        Some(("account", sub_matches)) => match sub_matches.get_flag("all") {
                            true => deactivate_all_accounts().await?,
                            false => {
                                for account_id in sub_matches.get_many::<i64>("account_id").unwrap()
                                // arg_required_else_help has been set to true
                                {
                                    deactivate_account(*account_id).await?;
                                }
                            }
                        },
                        Some(("set", sub_matches)) => match sub_matches.get_flag("all") {
                            true => deactivate_all_sets().await?,
                            false => {
                                for set_id in sub_matches.get_many::<i64>("set_id").unwrap()
                                // arg_required_else_help has been set to true
                                {
                                    deactivate_set(*set_id).await?;
                                }
                            }
                        },
                        Some(("up", sub_matches)) => match sub_matches.get_flag("all") {
                            true => deactivate_all_ups().await?,
                            false => {
                                for up_id in sub_matches.get_many::<i64>("up_id").unwrap()
                                // arg_required_else_help has been up to true
                                {
                                    deactivate_up(*up_id).await?;
                                }
                            }
                        },
                        _ => unreachable!(),
                    },
                    Some(("fetch", sub_matches)) => fetch(sub_matches.get_flag("prune")).await?,
                    Some(("status", sub_matches)) => match sub_matches.subcommand() {
                        Some(("set", sub_matches)) => {
                            let name = sub_matches
                                .get_one::<String>("name")
                                .context("Folder is Not Allow")?;
                            let path = sub_matches
                                .get_one::<String>("path")
                                .context("Not folder path")?;

                            let active = sub_matches.get_flag("switch");

                            status_set(name, path, active).await?;
                        }
                        _ => status().await?,
                    },
                    Some(("like", sub_matches)) => {
                        like(sub_matches.get_many("avids").unwrap().copied().collect()).await?
                    }
                    Some(("pull", _)) => pull().await?,
                    Some(("clone", sub_matches)) => {
                        for bvid in sub_matches
                            .get_many::<String>("bv_id")
                            .context("Not one active status folder")?
                        {
                            only_download(bvid).await?;
                        }
                    }
                    Some(("resp", _)) => {
                        error!("you already in resp state");
                    }
                    _other => {
                        error!("There is a lawer commands");
                    }
                }
            }
        }
        Ok(())
    }
}
