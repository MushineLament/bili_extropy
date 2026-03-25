use anyhow::Result;
use sea_orm::{EntityTrait as _, IntoActiveModel as _};

use super::Db;
use crate::entity::up_media;

impl Db {
    pub async fn upsert_media_ups(
        &self,
        media_ups: impl IntoIterator<Item = up_media::UpMediaModel>,
    ) -> Result<()> {
        up_media::UpMediaEntity::insert_many(media_ups.into_iter().map(|m| m.into_active_model()))
            .on_conflict_do_nothing()
            .exec_without_returning(&self.db)
            .await?;
        Ok(())
    }
}
