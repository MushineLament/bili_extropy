use clap::{Arg, ArgAction, Command, value_parser};

use clap_complete::Shell;
use tracing_subscriber::EnvFilter;

use crate::{
    command::{
        account::{command_auth, execute_auth},
        fetch::{command_fetch, execute_fetch},
        list::{command_list, execute_list},
        pull::{command_pull, execute_pull},
        status::{command_status, execute_status},
    },
    version::VERSION,
};

pub const APP_NAME: &str = "bili_extropy_ecs";

pub const HISTROY: &str = ".temp/.histroy.txt";

#[derive(Debug)]
pub struct Console(pub Command);

impl Console {
    pub fn new() -> Self {
        Self(
            Command::new(APP_NAME)
                .arg_required_else_help(true)
                .version(VERSION)
                .subcommands([
                    command_auth(),
                    command_status(),
                    command_list(),
                    command_fetch(),
                    command_pull(),
                    Command::new("like")
                        .about("Like medias")
                        .arg_required_else_help(true)
                        .args([Arg::new("avids")
                            .help("The avids to like")
                            .value_parser(value_parser!(i64))
                            .action(ArgAction::Append)]),
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

    /// Parse the commands and args, return the Event to trigger.
    pub fn run(mut self) {
        let matches = self.0.get_matches_mut();

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
                match matches.get_flag("verbose") {
                    true => {
                        let filter = EnvFilter::from_default_env()
                            .add_directive("fav=debug".parse().unwrap());
                        tracing_subscriber::fmt()
                            .with_env_filter(filter)
                            .with_thread_ids(true)
                            .with_line_number(true)
                            .init();
                    }
                    false => {
                        let filter = EnvFilter::from_default_env()
                            .add_directive("fav=info".parse().unwrap());
                        tracing_subscriber::fmt()
                            .with_target(false)
                            .with_env_filter(filter)
                            .without_time()
                            .init();
                    }
                }

                match sub_cmd {
                    Some(("auth", sub_matches)) => execute_auth(sub_matches),
                    Some(("status", sub_matches)) => execute_status(sub_matches),
                    Some(("list", sub_matches)) => execute_list(sub_matches),
                    Some(("pull", sub_matches)) => execute_pull(sub_matches),
                    Some(("fetch", sub_matches)) => execute_fetch(sub_matches),
                    // Some(("list", sub_matches)) => match sub_matches.subcommand() {
                    //     Some(("account", _)) => list_accounts().await?,
                    //     Some(("set", _)) => list_sets().await?,
                    //     Some(("up", _)) => list_ups().await?,
                    //     Some(("media", _)) => list_medias().await?,
                    //     _ => unreachable!(),
                    // },
                    // Some(("activate", sub_matches)) => match sub_matches.subcommand() {
                    //     Some(("account", sub_matches)) => match sub_matches.get_flag("all") {
                    //         true => activate_all_accounts().await?,
                    //         false => {
                    //             for account_id in sub_matches.get_many::<i64>("account_id").unwrap()
                    //             // arg_required_else_help has been set to true
                    //             {
                    //                 activate_account(*account_id).await?;
                    //             }
                    //         }
                    //     },
                    //     Some(("set", sub_matches)) => match sub_matches.get_flag("all") {
                    //         true => activate_all_sets().await?,
                    //         false => {
                    //             for set_id in sub_matches.get_many::<i64>("set_id").unwrap()
                    //             // arg_required_else_help has been set to true
                    //             {
                    //                 activate_set(*set_id).await?;
                    //             }
                    //         }
                    //     },
                    //     Some(("up", sub_matches)) => match sub_matches.get_flag("all") {
                    //         true => activate_all_ups().await?,
                    //         false => {
                    //             for up_id in sub_matches.get_many::<i64>("up_id").unwrap()
                    //             // arg_required_else_help has been up to true
                    //             {
                    //                 activate_up(*up_id).await?;
                    //             }
                    //         }
                    //     },
                    //     _ => unreachable!(),
                    // },
                    // Some(("deactivate", sub_matches)) => match sub_matches.subcommand() {
                    //     Some(("account", sub_matches)) => match sub_matches.get_flag("all") {
                    //         true => deactivate_all_accounts().await?,
                    //         false => {
                    //             for account_id in sub_matches.get_many::<i64>("account_id").unwrap()
                    //             // arg_required_else_help has been set to true
                    //             {
                    //                 deactivate_account(*account_id).await?;
                    //             }
                    //         }
                    //     },
                    //     Some(("set", sub_matches)) => match sub_matches.get_flag("all") {
                    //         true => deactivate_all_sets().await?,
                    //         false => {
                    //             for set_id in sub_matches.get_many::<i64>("set_id").unwrap()
                    //             // arg_required_else_help has been set to true
                    //             {
                    //                 deactivate_set(*set_id).await?;
                    //             }
                    //         }
                    //     },
                    //     Some(("up", sub_matches)) => match sub_matches.get_flag("all") {
                    //         true => deactivate_all_ups().await?,
                    //         false => {
                    //             for up_id in sub_matches.get_many::<i64>("up_id").unwrap()
                    //             // arg_required_else_help has been up to true
                    //             {
                    //                 deactivate_up(*up_id).await?;
                    //             }
                    //         }
                    //     },
                    //     _ => unreachable!(),
                    // },
                    // Some(("fetch", sub_matches)) => fetch(sub_matches.get_flag("prune")).await?,
                    // Some(("like", sub_matches)) => {
                    //     like(sub_matches.get_many("avids").unwrap().copied().collect()).await?
                    // }
                    // Some(("pull", _)) => pull().await?,
                    _ => unreachable!(),
                }
            }
        }
    }
}
