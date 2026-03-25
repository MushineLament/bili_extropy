use anyhow::Result;
use sea_orm::{EntityTrait, IntoActiveModel, sea_query::OnConflict};

use crate::{db::Db, entity::media};

impl Db {
    pub async fn download_upsert_medias(
        &self,
        medias: impl IntoIterator<Item = media::MediaModel>,
    ) -> Result<()> {
        media::MediaEntity::insert_many(medias.into_iter().map(|m| m.into_active_model()))
            .on_conflict(
                OnConflict::column(media::Column::BvId)
                    .update_columns([media::Column::Title, media::Column::Id, media::Column::Type])
                    .to_owned(),
            )
            .exec_without_returning(&self.db)
            .await?;
        Ok(())
    }
}
