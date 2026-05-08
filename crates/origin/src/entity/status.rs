use std::borrow::Cow;

use sea_orm::entity::prelude::*;

// ========== 向后兼容别名 ==========
pub use ActiveModel as StatusActiveModel;
pub use Entity as StatusEntity;
pub use Model as StatusModel;

use crate::table::ToTableRecord;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Hash)]
#[sea_orm(table_name = "status")]
pub struct Model {
    /// # example
    /// ```
    /// let status = ActiveModel {
    ///     id: ActiveValue::NotSet,   // 让数据库生成
    ///     name: Set("example".to_owned()),
    ///     path: Set("/path".to_owned()),
    ///     state: Set("active".to_owned()),
    /// };
    ///
    /// ```
    /// # or
    /// ```
    /// let model = Model {
    ///     id: 0,  // 临时值，会被覆盖
    ///     name: "example".to_string(),
    ///     path: "/path".to_string(),
    ///     state: "active".to_string(),
    /// };
    /// let mut active = model.into_active_model();
    /// active.id = ActiveValue::NotSet;
    /// ```
    ///
    ///
    ///
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub name: String,
    pub path: String,
    pub state: String,
}

impl ToTableRecord<4> for StatusModel {
    fn to_record(&self) -> [Cow<'_, str>; 4] {
        [
            Cow::Owned(self.id.to_string()),
            Cow::Borrowed(&self.name),
            Cow::Borrowed(&self.path),
            Cow::Borrowed(&self.state),
        ]
    }
}

impl ToTableRecord<4> for &StatusModel {
    fn to_record(&self) -> [Cow<'_, str>; 4] {
        [
            Cow::Owned(self.id.to_string()),
            Cow::Borrowed(&self.name),
            Cow::Borrowed(&self.path),
            Cow::Borrowed(&self.state),
        ]
    }
}

// 关系定义（如果有）
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::entity::status_downloadrule::Entity")]
    StatusDownloadrule,
}

impl Related<crate::entity::downloadrule::DownloadruleEntity> for Entity {
    fn to() -> RelationDef {
        // 通过中间表进行多对多关联
        super::status_downloadrule::Relation::Downloadrule.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::status_downloadrule::Relation::Status.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
