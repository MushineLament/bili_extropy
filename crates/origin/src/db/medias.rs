use anyhow::Result;
use sea_orm::EntityTrait as _;

use crate::{db::Db, entity::media};

impl Db {
    pub async fn all_medias(&self) -> Result<Vec<media::MediaModel>> {
        media::MediaEntity::find()
            .all(&self.db)
            .await
            .map_err(Into::into)
    }
}
