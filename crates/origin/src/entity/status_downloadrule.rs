use std::borrow::Cow;

use sea_orm::entity::prelude::*;

// ========== 向后兼容别名 ==========
pub use ActiveModel as StatusDownloadruleActiveModel;
pub use Entity as StatusDownloadruleEntity;
pub use Model as StatusDownloadruleModel;

use crate::table::ToTableRecord;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Hash)]
#[sea_orm(table_name = "status_downloadrule")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub status_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub rule_id: i64,
}

impl ToTableRecord<2> for Model {
    fn to_record(&self) -> [Cow<'_, str>; 2] {
        [
            Cow::Owned(self.status_id.to_string()),
            Cow::Owned(self.rule_id.to_string()),
        ]
    }
}

impl ToTableRecord<2> for &Model {
    fn to_record(&self) -> [Cow<'_, str>; 2] {
        [
            Cow::Owned(self.status_id.to_string()),
            Cow::Owned(self.rule_id.to_string()),
        ]
    }
}

// 关系定义（如果有）
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::status::Entity",
        from = "Column::StatusId",
        to = "super::status::Column::Id"
    )]
    Status,
    #[sea_orm(
        belongs_to = "super::downloadrule::Entity",
        from = "Column::RuleId",
        to = "super::downloadrule::Column::Id"
    )]
    Downloadrule,
}

impl Related<super::status::StatusEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Status.def()
    }
}

impl Related<super::downloadrule::DownloadruleEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Downloadrule.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
