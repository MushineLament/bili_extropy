use anyhow::Result;
use sea_orm::{EntityTrait as _, IntoActiveModel as _};

use super::Db;
use crate::entity::collection_media;

impl Db {
    pub async fn upsert_media_sets(
        &self,
        media_sets: impl IntoIterator<Item = collection_media::CollectionMediaModel>,
    ) -> Result<()> {
        collection_media::CollectionMediaEntity::insert_many(media_sets.into_iter().map(|m| m.into_active_model()))
            .on_conflict_do_nothing()
            .exec_without_returning(&self.db)
            .await?;
        Ok(())
    }
}
