use std::{path::Path, sync::LazyLock};

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use sea_orm::EntityTrait;
use tracing::error;

use crate::{
    RUNTIMER,
    components::status::Status,
    db::DB,
    entity::status::{self},
    table::ToTable,
};

pub const STATUS: LazyLock<Status> =
    LazyLock::new(|| RUNTIMER.block_on(Status::load(&DB)).expect("待完善信息"));

pub fn command_status() -> Command {
    Command::new("status")
        .about("Download Floders")
        .arg_required_else_help(true)
        .subcommands([
            Command::new("list").about("List Download Folders"),
            Command::new("add")
                .about("Add Download Folders")
                .arg_required_else_help(true)
                .args([
                    // 第一个位置参数：PATH
                    Arg::new("path")
                        .help("Download path")
                        .value_name("PATH") // 帮助里显示 <PATH>
                        .required(true), // 尖括号 = 必填
                    // 第二个位置参数：FOLDER
                    Arg::new("folder")
                        .help("Download folder name")
                        .value_name("FOLDER")
                        .required(false),
                    // option：--state <STATE>
                    Arg::new("state")
                        .long("state")
                        .help("Set state")
                        .value_name("STATE")
                        .value_parser(["Active", "Inactive", "Unreachable"])
                        .action(ArgAction::Set)
                        .default_value("Active"),
                ]),
            Command::new("remove")
                .about("Remove Download Folders")
                .arg_required_else_help(true)
                .args([
                    // 第一个位置参数：PATH
                    Arg::new("id")
                        .help("Download path id")
                        .value_name("ID")
                        .value_parser(value_parser!(i32))
                        .required(true), // 尖括号 = 必填
                ]),
        ])
}

pub fn execute_status(arg: &ArgMatches) {
    match arg.subcommand() {
        Some(("list", _)) => {
            let table = STATUS
                .0
                .iter()
                .table_head(["id", "folder name", "path", "state"]);
            println!("\n{}\nrows: {}", table, table.count_rows() - 1);
        }
        Some(("add", subarg)) => {
            let path = subarg
                .get_one::<String>("path")
                .map(Path::new)
                .unwrap_or(Path::new("./"));
            let folder = subarg.get_one::<String>("folder");
            let state = subarg
                .get_one::<String>("state")
                .map(String::as_str)
                .unwrap_or("Unreachable");

            let (parent, folder) = match folder {
                Some(folder) => (path, folder.as_str()),
                None => {
                    let Some(parent) = path.parent() else {
                        println!("get folder parent path fail");
                        return;
                    };
                    let Some(folder) = path.file_stem().and_then(|f| f.to_str()) else {
                        println!("get file stem fail");
                        return;
                    };
                    (parent, folder)
                }
            };

            let Some(parent) = parent.to_str().map(|s| if s.is_empty() { "." } else { s }) else {
                println!("get folder parent path str fail");
                return;
            };

            if let Err(err) = RUNTIMER.block_on(
                status::StatusEntity::insert(status::StatusActiveModel {
                    id: sea_orm::ActiveValue::NotSet,
                    name: sea_orm::ActiveValue::Set(folder.to_owned()),
                    path: sea_orm::ActiveValue::Set(parent.to_owned()),
                    state: sea_orm::ActiveValue::Set(state.to_string()),
                })
                .exec(&DB.0),
            ) {
                error!("insert status failed: {err:?}");
            };
        }
        Some(("remove", subarg)) => {
            let Some(id) = subarg.get_one::<i32>("id") else {
                println!("Not has any id");
                return;
            };

            if let Err(err) = RUNTIMER.block_on(status::StatusEntity::delete_by_id(*id).exec(&DB.0))
            {
                error!("remove status failed: {err:?}");
            };
        }

        _ => unreachable!(),
    }
}
