use sea_orm::{DbErr, EntityTrait as _};
use strum::{Display, EnumString};

use crate::{
    db::Db,
    entity::status::{self, StatusModel},
};

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display)]
pub enum StatusState {
    Active,
    Inactive,
    Exclusive,
}

#[derive(Debug, Clone)]
pub struct Status(pub Vec<StatusModel>);

impl Status {
    // pub fn legal_path(&self) -> PathBuf {
    //     Path::new(&self.path).join(&self.name)
    // }

    pub async fn load(db: &Db) -> Result<Self, DbErr> {
        Ok(Self(status::StatusEntity::find().all(&db.0).await?))
    }
}
