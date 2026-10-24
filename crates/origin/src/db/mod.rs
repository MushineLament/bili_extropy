use std::{
    fs,
    io::ErrorKind,
    ops::{Deref, DerefMut},
    path::Path,
    sync::LazyLock,
};

use migration::{Migrator, MigratorTrait as _};
use sea_orm::{Database, DatabaseConnection, DbErr};

use crate::RUNTIMER;

pub mod status;

pub const DB_PATH: &str = ".bili_extropy/bili_extropy.db";

pub const DB: LazyLock<Db> = LazyLock::new(|| Db::connect().expect("数据库连接失败"));

#[derive(Debug)]
pub enum LinkDbErr {
    DbPathNoExist,
    DbPathCreateFail,
    NotAFileDb,
    DbErr(DbErr),
}

#[derive(Debug, Clone)]
pub struct Db(pub DatabaseConnection);

impl Db {
    pub fn connect() -> Result<Self, LinkDbErr> {
        RUNTIMER.block_on(async {
            let db_path = Path::new(DB_PATH);

            let Some(parent) = db_path.parent() else {
                return Err(LinkDbErr::DbPathNoExist);
            };

            let db = if !db_path.exists() {
                if let Err(err) = fs::create_dir_all(parent)
                    && err.kind() != ErrorKind::AlreadyExists
                {
                    return Err(LinkDbErr::DbPathCreateFail);
                };
                Database::connect("sqlite://.bili_extropy/bili_extropy.db?mode=rwc")
            } else if db_path.is_file() {
                Database::connect("sqlite://.bili_extropy/bili_extropy.db?mode=rw")
            } else {
                return Err(LinkDbErr::NotAFileDb);
            };

            let db = db.await.map_err(|err| LinkDbErr::DbErr(err))?;

            Migrator::up(&db, None)
                .await
                .map_err(|err| LinkDbErr::DbErr(err))?;

            Ok(Self(db))
        })
    }
}

impl Deref for Db {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Db {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
