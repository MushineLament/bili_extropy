use std::borrow::Cow;

use sea_orm::entity::prelude::*;

// ========== 向后兼容别名 ==========
pub type DownloadtaskMediasActiveModel = ActiveModel;
pub type DownloadtaskMediasEntity = Entity;
pub type DownloadtaskMediasModel = Model;

use crate::table::ToTableRecord;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Hash)]
#[sea_orm(table_name = "downloadtask_medias")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub task_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub media_id: i64,
    pub state: String,
}

impl ToTableRecord<3> for Model {
    fn to_record(&self) -> [Cow<'_, str>; 3] {
        [
            Cow::Owned(self.task_id.to_string()),
            Cow::Owned(self.media_id.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

impl ToTableRecord<3> for &Model {
    fn to_record(&self) -> [Cow<'_, str>; 3] {
        [
            Cow::Owned(self.task_id.to_string()),
            Cow::Owned(self.media_id.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

// 关系定义（如果有）
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::downloadtask::Entity",
        from = "Column::TaskId",
        to = "super::downloadtask::Column::Id"
    )]
    TaskId,
    #[sea_orm(
        belongs_to = "super::media::Entity",
        from = "Column::MediaId",
        to = "super::media::Column::Aid"
    )]
    MediaId,
}

impl Related<super::downloadtask::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TaskId.def()
    }
}

impl Related<super::media::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MediaId.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
