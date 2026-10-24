use crate::{
    db::Db,
    entity::status::{self, StatusEntity, StatusModel},
    state::CollectionState,
};

use sea_orm::{
    ColumnTrait, DbErr, EntityTrait, IntoActiveModel, QueryFilter, sea_query::OnConflict,
};

impl Db {
    pub async fn upsert_status(
        &self,
        medias: impl IntoIterator<Item = StatusModel>,
    ) -> Result<(), DbErr> {
        StatusEntity::insert_many(medias.into_iter().map(|m| m.into_active_model()))
            .on_conflict(
                OnConflict::column(status::Column::Id)
                    .update_columns([
                        status::Column::Name,
                        status::Column::Path,
                        status::Column::State,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&self.0)
            .await?;
        Ok(())
    }

    /// 获取激活的下载目录
    /// 当存在多个目录时，会
    pub async fn get_active_status(&self) -> Result<Vec<StatusModel>, DbErr> {
        status::StatusEntity::find()
            .filter(status::Column::State.eq(CollectionState::Active))
            .all(&self.0)
            .await
    }

    pub async fn all_status(&self) -> Result<Vec<StatusModel>, DbErr> {
        status::StatusEntity::find().all(&self.0).await
    }

    pub async fn get_status_by_id(&self, id: i64) -> Result<Option<StatusModel>, DbErr> {
        status::StatusEntity::find_by_id(id).one(&self.0).await
    }

    pub async fn get_status_by_folder(
        &self,
        name: &str,
        path: &str,
    ) -> Result<Option<StatusModel>, DbErr> {
        status::StatusEntity::find()
            .filter(status::Column::Name.eq(name))
            .filter(status::Column::Path.eq(path))
            .one(&self.0)
            .await
    }

    pub async fn activate_status_by_id(&self, id: i64) -> Result<StatusModel, DbErr> {
        let active_model = status::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(id), // 主键不变
            state: sea_orm::ActiveValue::Set(CollectionState::Active.to_string()),
            ..Default::default()
        };

        status::StatusEntity::update(active_model)
            .exec(&self.0)
            .await
    }

    pub async fn deactivate_status_by_id(&self, id: i64) -> Result<(), DbErr> {
        let active_model = status::ActiveModel {
            id: sea_orm::ActiveValue::Unchanged(id),
            state: sea_orm::ActiveValue::Set(CollectionState::Inactive.to_string()),
            ..Default::default()
        };

        status::StatusEntity::update(active_model)
            .exec(&self.0)
            .await?;

        Ok(())
    }
}
