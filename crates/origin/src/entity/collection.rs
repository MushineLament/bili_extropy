//! `SeaORM` Entity, 已手动转换为 DeriveEntityModel 现代化写法

use std::borrow::Cow;

use sea_orm::entity::prelude::*;

use crate::table::ToTableRecord;

pub const COLLECTION: &str = "Collection";

// ========== 向后兼容别名 ==========
pub use Entity as CollectionEntity;
pub use Model as CollectionModel;

// ========== 实体模型 ==========

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "collection")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub collection_id: i64,
    pub name: String,
    pub count: i64,
}

// ========== 关系定义 ==========

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::entity::collection_media::CollectionMediaEntity")]
    MediaCollection,
    #[sea_orm(has_many = "crate::entity::account_collection::AccountCollectionEntity")]
    SetAccount,
}

impl Related<crate::entity::collection_media::CollectionMediaEntity> for Entity {
    fn to() -> RelationDef {
        Relation::MediaCollection.def()
    }
}

impl Related<crate::entity::account_collection::AccountCollectionEntity> for Entity {
    fn to() -> RelationDef {
        Relation::SetAccount.def()
    }
}

impl Related<crate::entity::account::AccountEntity> for Entity {
    fn to() -> RelationDef {
        crate::entity::account_collection::Relation::Account.def()
    }
    fn via() -> Option<RelationDef> {
        Some(crate::entity::account_collection::Relation::Set.def().rev())
    }
}

impl Related<crate::entity::media::MediaEntity> for Entity {
    fn to() -> RelationDef {
        crate::entity::collection_media::Relation::Media.def()
    }
    fn via() -> Option<RelationDef> {
        Some(crate::entity::collection_media::Relation::Set.def().rev())
    }
}

impl Related<crate::entity::status::StatusEntity> for Entity {
    fn to() -> RelationDef {
        crate::entity::collection::Relation::MediaCollection.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            crate::entity::collection::Relation::MediaCollection
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}

// ========== 表格显示 trait 实现 ==========

impl ToTableRecord<3> for Model {
    fn to_record(&self) -> [Cow<'_, str>; 3] {
        [
            Cow::Owned(self.collection_id.to_string()),
            Cow::Borrowed(&self.name),
            Cow::Owned(self.count.to_string()),
        ]
    }
}

use serde::Deserialize;

use crate::entity::media::{MediaCollection, MediaUp};

#[derive(Debug, Deserialize)]
pub struct ListUpperCollectResp {
    pub data: ListUpperCollectData,
}

#[derive(Debug, Deserialize)]
pub struct ListUpperCollectData {
    pub list: Vec<Collection>,
}

#[derive(Debug, Deserialize)]
pub struct Collection {
    pub id: i64,
    pub media_count: i64,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct InSetResp {
    pub data: InSetData,
}

#[derive(Debug, Deserialize)]
pub struct InSetData {
    pub medias: Vec<MediaCollection>,
}

#[derive(Debug, Deserialize)]
pub struct InUpResp {
    pub data: InUpData,
}

#[derive(Debug, Deserialize)]
pub struct InUpData {
    pub list: InUpList,
}

#[derive(Debug, Deserialize)]
pub struct InUpList {
    pub vlist: Vec<MediaUp>,
}
