use std::{fs, io::ErrorKind, path::Path};

use anyhow::Result;
use bevy::{app::Plugin, ecs::resource::Resource};
use migration::{Migrator, MigratorTrait as _};
use sea_orm::{Database, DatabaseConnection};
use tokio::runtime::Runtime;

pub mod status;

pub const DB_PATH: &str = ".bili_extropy_ecs/bili_extropy_ecs.db";

#[derive(Debug, Clone, Resource)]
pub struct Db {
    pub db: DatabaseConnection,
}

impl Db {
    pub fn connect() -> Result<Self> {
        // 创建一个 Tokio 运行时
        let rt =
            Runtime::new().map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;

        rt.block_on(async {
            let db_path = Path::new(DB_PATH);

            let Some(parent) = db_path.parent() else {
                return Err(anyhow::anyhow!("get path parent error: {:?}", DB_PATH));
            };

            let db = if !db_path.exists() {
                if let Err(err) = fs::create_dir_all(parent)
                    && err.kind() != ErrorKind::AlreadyExists
                {
                    return Err(anyhow::anyhow!(
                        "create \"{:?}\" dir err: {:?}",
                        DB_PATH,
                        err
                    ));
                };
                Database::connect("sqlite://.bili_extropy_ecs/bili_extropy_ecs.db?mode=rwc")
            } else if db_path.is_file() {
                Database::connect("sqlite://.bili_extropy_ecs/bili_extropy_ecs.db?mode=rw")
            } else {
                return Err(anyhow::anyhow!("db path error: {:?}", db_path));
            };

            let db = db
                .await
                .map_err(|err| anyhow::anyhow!("Failed to connect db:{:?}", err))?;

            Migrator::up(&db, None)
                .await
                .map_err(|err| anyhow::anyhow!("Failed to update db tables:{:?}", err))?;

            Ok(Self { db })
        })
    }
}

pub struct DbPlugin;

impl Plugin for DbPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        let db = Db::connect().unwrap();
        app.insert_resource(db);
    }
}
