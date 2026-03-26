use crate::{
    db::Db,
    entity::status::{self, StatusEntity, StatusModel},
    state::SetState,
};
use anyhow::{Context, Result};
use sea_orm::{ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, sea_query::OnConflict};

impl Db {
    pub async fn upsert_status(&self, medias: impl IntoIterator<Item = StatusModel>) -> Result<()> {
        StatusEntity::insert_many(medias.into_iter().map(|m| m.into_active_model()))
            .on_conflict(
                OnConflict::column(status::Column::Id)
                    .update_columns([
                        status::Column::Name,
                        status::Column::Path,
                        status::Column::CollectionId,
                        status::Column::State,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&self.db)
            .await?;
        Ok(())
    }

    /// 获取激活的下载目录
    /// 当存在多个目录时，会
    pub async fn get_status(&self) -> Result<StatusModel> {
        status::StatusEntity::find()
            .filter(status::Column::State.eq(SetState::Active))
            .one(&self.db)
            .await?
            .context("Not Active Folder")
    }

    pub async fn all_status(&self) -> Result<Vec<StatusModel>> {
        Ok(status::StatusEntity::find().all(&self.db).await?)
    }
}
