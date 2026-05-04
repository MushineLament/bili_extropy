use std::borrow::Cow;

use sea_orm::entity::prelude::*;

// ========== 向后兼容别名 ==========
pub use ActiveModel as DownloadtaskActiveModel;
pub use Entity as DownloadtaskEntity;
pub use Model as DownloadtaskModel;

use crate::table::ToTableRecord;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Hash)]
#[sea_orm(table_name = "downloadtask")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub type_id: String,
    pub generic_id: i64,
    pub state: String,
}

impl ToTableRecord<4> for Model {
    fn to_record(&self) -> [Cow<'_, str>; 4] {
        [
            Cow::Owned(self.id.to_string()),
            Cow::Borrowed(&self.type_id),
            Cow::Owned(self.generic_id.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

impl ToTableRecord<4> for &Model {
    fn to_record(&self) -> [Cow<'_, str>; 4] {
        [
            Cow::Owned(self.id.to_string()),
            Cow::Borrowed(&self.type_id),
            Cow::Owned(self.generic_id.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

// 关系定义（如果有）
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
