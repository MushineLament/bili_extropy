use sea_orm::entity::prelude::*;
use serde::Deserialize;
use url::Url;

use crate::{
    entity::{BvId, MediaAid, Title, UpperCid, up::Upper},
    table::ToTableRecord,
};

pub use self::Entity as MediaEntity;
pub use self::Model as MediaModel;

#[derive(Clone, Debug, PartialEq, Eq, Hash, DeriveEntityModel, Deserialize)]
#[sea_orm(table_name = "media")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[serde(alias = "aid")]
    pub aid: MediaAid,

    #[sea_orm(column_type = "String(StringLen::None)")]
    #[serde(rename = "bvid")]
    pub bv_id: BvId,

    /// 作者的id
    pub cid: UpperCid,

    #[sea_orm(column_type = "String(StringLen::None)")]
    pub title: Title,

    #[sea_orm(column_type = "String(StringLen::N(32))")]
    #[serde(default)]
    pub r#type: String,

    #[sea_orm(default_value = "normal")]
    #[serde(skip_deserializing, default = "default_state")]
    pub state: String,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub pic: Option<Url>,
}

fn default_state() -> String {
    "normal".to_string()
}

// 关系定义 - 使用具体结构体名称
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::entity::collection_media::CollectionMediaEntity")]
    MediaSet,
    #[sea_orm(has_many = "crate::entity::up_media::UpMediaEntity")]
    MediaUp,
}

impl Related<crate::entity::collection_media::CollectionMediaEntity> for Entity {
    fn to() -> RelationDef {
        Relation::MediaSet.def()
    }
}

impl Related<crate::entity::up_media::UpMediaEntity> for Entity {
    fn to() -> RelationDef {
        Relation::MediaUp.def()
    }
}

impl Related<crate::entity::collection::CollectionEntity> for Entity {
    fn to() -> RelationDef {
        crate::entity::collection_media::Relation::Set.def()
    }
    fn via() -> Option<RelationDef> {
        Some(crate::entity::collection_media::Relation::Media.def().rev())
    }
}

impl Related<crate::entity::up::Entity> for Entity {
    fn to() -> RelationDef {
        crate::entity::up_media::Relation::Up.def()
    }
    fn via() -> Option<RelationDef> {
        Some(crate::entity::up_media::Relation::Media.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl ToTableRecord<5> for Model {
    fn to_record(&self) -> [Cow<'_, str>; 5] {
        [
            Cow::Owned(self.aid.to_string()),
            Cow::Borrowed(&self.bv_id),
            Cow::Borrowed(&self.title),
            Cow::Owned(self.r#type.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

impl ToTableRecord<5> for &Model {
    fn to_record(&self) -> [Cow<'_, str>; 5] {
        [
            Cow::Owned(self.aid.to_string()),
            Cow::Borrowed(&self.bv_id),
            Cow::Borrowed(&self.title),
            Cow::Owned(self.r#type.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

// ========== 以下为 API 响应结构体，保持不变 ==========

use core::fmt;
use std::borrow::Cow;

#[derive(Debug, Deserialize)]
pub struct MediaInfoSingle {
    pub code: i64,
    pub data: Option<Media>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MediaInfoResp {
    pub code: i64,
    pub data: Option<MediaInfoData>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MediaInfoData {
    pub owner: Upper,
    pub pages: Vec<Page>,
    pub staff: Option<Vec<Upper>>,
    pub cid: UpperCid,
}

#[derive(Debug, Deserialize)]
pub struct Page {
    pub cid: UpperCid,
    pub page: i64,
    pub part: String,
}

#[derive(Debug, Deserialize)]
pub struct Media {
    pub aid: MediaAid,
    pub bvid: BvId,
    pub cid: UpperCid,
    pub title: Title,
    #[serde(default)]
    pub r#type: MediaType,
    pub pic: Url,
}

#[derive(Debug, Deserialize)]
pub struct MediaUp {
    #[serde(alias = "aid")]
    pub id: i64,
    #[serde(rename = "bvid")]
    pub bv_id: String,
    pub mid: i64,
    pub title: String,
    #[serde(default)]
    pub r#type: MediaType,
}

#[derive(Debug, Deserialize)]
pub struct MediaCollection {
    pub id: i64,
    #[serde(rename = "bvid")]
    pub bv_id: String,
    pub upper: Upper,
    pub title: String,
    #[serde(default)]
    pub r#type: MediaType,
    pub cover: Url,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[repr(u8)]
#[serde(from = "u8")]
pub enum MediaType {
    #[default]
    Video = 2,
    Audio = 12,
    Collection = 21,
}

impl From<u8> for MediaType {
    fn from(value: u8) -> Self {
        match value {
            2 => Self::Video,
            12 => Self::Audio,
            21 => Self::Collection,
            _ => Self::Video,
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Video => write!(f, "Video"),
            Self::Audio => write!(f, "Audio"),
            Self::Collection => write!(f, "Collection"),
        }
    }
}
