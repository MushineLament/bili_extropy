use sea_orm::entity::prelude::*;

use crate::table::ToTableRecord;

type Entity = MediaEntity;
type Model = MediaModel;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct MediaEntity;

impl EntityName for MediaEntity {
    fn table_name(&self) -> &str {
        "media"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Hash)]
pub struct MediaModel {
    pub aid: i64,
    pub bv_id: String,
    /// 视频up主的cid
    pub cid: i64,
    pub title: String,
    pub r#type: String,
    pub state: String,
}

impl ToTableRecord<5> for MediaModel {
    fn to_record(self) -> [String; 5] {
        [
            self.aid.to_string(),
            self.bv_id,
            self.title,
            self.r#type.to_string(),
            self.state.to_string(),
        ]
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Aid,
    BvId,
    Title,
    Cid,
    Type,
    State,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Aid,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = i64;
    fn auto_increment() -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    MediaSet,
    MediaUp,
}

impl ColumnTrait for Column {
    type EntityName = MediaEntity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Aid => ColumnType::BigInteger.def().unique(),
            Self::BvId => ColumnType::String(StringLen::None).def(),
            Self::Title => ColumnType::String(StringLen::None).def(),
            Self::Type => ColumnType::custom("enum_text").def(),
            Self::State => ColumnType::custom("enum_text").def(),
            Self::Cid => ColumnType::BigInteger.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::MediaSet => {
                MediaEntity::has_many(super::collection_media::CollectionMediaEntity).into()
            }
            Self::MediaUp => MediaEntity::has_many(super::up_media::UpMediaEntity).into(),
        }
    }
}

impl Related<super::collection_media::CollectionMediaEntity> for MediaEntity {
    fn to() -> RelationDef {
        Relation::MediaSet.def()
    }
}

impl Related<super::up_media::UpMediaEntity> for MediaEntity {
    fn to() -> RelationDef {
        Relation::MediaUp.def()
    }
}

impl Related<super::collection::CollectionEntity> for MediaEntity {
    fn to() -> RelationDef {
        super::collection_media::Relation::Set.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::collection_media::Relation::Media.def().rev())
    }
}

impl Related<super::up::Entity> for MediaEntity {
    fn to() -> RelationDef {
        super::up_media::Relation::Up.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::up_media::Relation::Media.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
