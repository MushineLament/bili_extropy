use crate::{
    db::Db,
    entity::status::{self, StatusEntity, StatusModel},
    state::SetState,
};
use anyhow::{Context, Ok, Result};
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
    pub async fn get_active_status(&self) -> Result<StatusModel> {
        status::StatusEntity::find()
            .filter(status::Column::State.eq(SetState::Active))
            .one(&self.db)
            .await?
            .context("Not or has many active active folder")
    }

    pub async fn all_status(&self) -> Result<Vec<StatusModel>> {
        Ok(status::StatusEntity::find().all(&self.db).await?)
    }

    pub async fn get_status_by_id(&self, id: i64) -> Result<StatusModel> {
        status::StatusEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .context(anyhow::anyhow!("Has not id status<{}>", id))
    }
    pub async fn get_status_by_folder(&self, name: &str, path: &str) -> Result<StatusModel> {
        status::StatusEntity::find()
            .filter(status::Column::Name.eq(name))
            .filter(status::Column::Path.eq(path))
            .one(&self.db)
            .await?
            .context("Not this Folder")
    }

    pub async fn activate_status_by_id(&self, id: i64) -> Result<()> {
        let active_model = status::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(Some(id)), // 主键不变
            state: sea_orm::ActiveValue::Set(SetState::Active.to_string()),
            ..Default::default()
        };

        status::StatusEntity::update(active_model)
            .exec(&self.db)
            .await?;

        Ok(())
    }

    pub async fn deactivate_status_by_id(&self, id: i64) -> Result<()> {
        let active_model = status::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(Some(id)),
            state: sea_orm::ActiveValue::Set(SetState::Inactive.to_string()),
            ..Default::default()
        };

        status::StatusEntity::update(active_model)
            .exec(&self.db)
            .await?;

        Ok(())
    }

    pub async fn set_status_collection(&self, id: i64, collection: i64) -> Result<StatusModel> {
        let active_model = status::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(Some(id)),
            collection_id: sea_orm::ActiveValue::set(Some(collection)),
            ..Default::default()
        };

        status::StatusEntity::update(active_model)
            .exec(&self.db)
            .await
            .context(anyhow::anyhow!("Update status<{}> collection id error", id))
    }
}
