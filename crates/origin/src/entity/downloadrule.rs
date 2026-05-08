use bevy::platform::collections::HashMap;
use chrono::NaiveDateTime;
use sea_orm::{ActiveValue, entity::prelude::*};
use std::borrow::Cow;

// ========== 向后兼容别名 ==========
pub type DownloadruleActiveModel = ActiveModel;
pub type DownloadruleEntity = Entity;
pub type DownloadruleModel = Model;

use crate::table::ToTableRecord;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Hash)]
#[sea_orm(table_name = "downloadrule")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub name: String,
    pub size: Option<i64>,
    pub relation_size: Option<String>,
    pub date: Option<NaiveDateTime>,
    pub relation_date: Option<String>,
    pub repeat: bool,
    pub state: String,
}

impl Model {
    pub fn default_relation_date(&self, other: NaiveDateTime) -> bool {
        let Some(relation_date) = &self.relation_date else {
            return true;
        };

        let Some(date) = self.date else {
            return true;
        };

        match relation_date.as_str() {
            "<" => date < other,
            "<=" => date <= other,
            "==" => date == other,
            ">=" => date >= other,
            ">" => date > other,
            _ => true,
        }
    }

    pub fn default_relation_size(&self, other: u64) -> bool {
        let Some(relation_size) = &self.relation_size else {
            return true;
        };

        let Some(sub) = self.size.map(|date| date as i64 - other as i64) else {
            return true;
        };

        match relation_size.as_str() {
            "<" => sub < 0,
            "<=" => sub <= 0,
            "==" => sub == 0,
            ">=" => sub >= 0,
            ">" => sub > 0,
            _ => true,
        }
    }
}

impl ActiveModel {
    pub fn from_argv_name(argv: &HashMap<String, Vec<String>>, name: String) -> ActiveModel {
        ActiveModel {
            id: argv
                .get("id")
                .map(|id| id.iter())
                .into_iter()
                .flatten()
                .find_map(|id| id.parse::<i64>().ok())
                .into_iter()
                .next()
                .map(|id| ActiveValue::Set(id))
                .unwrap_or(ActiveValue::NotSet),
            name: ActiveValue::Set(name),
            size: argv
                .get("size")
                .map(|id| id.iter())
                .into_iter()
                .flatten()
                .find_map(|id| id.parse::<i64>().ok())
                .into_iter()
                .next()
                .map(|id| ActiveValue::Set(Some(id)))
                .unwrap_or(ActiveValue::Set(None)),
            relation_size: argv
                .get("relationsize")
                .map(|relation| relation.iter())
                .into_iter()
                .flatten()
                .into_iter()
                .next()
                .map(|relation| ActiveValue::Set(Some(relation.to_string())))
                .unwrap_or(ActiveValue::Set(None)),
            date: argv
                .get("date")
                .map(|name| name.iter())
                .into_iter()
                .flatten()
                .into_iter()
                .next()
                .and_then(|name| NaiveDateTime::parse_from_str(name, "%Y-%m-%dT%H:%M:%S").ok())
                .map(|date| ActiveValue::Set(Some(date)))
                .unwrap_or(ActiveValue::Set(None)),
            relation_date: argv
                .get("relationdate")
                .map(|name| name.iter())
                .into_iter()
                .flatten()
                .into_iter()
                .next()
                .map(|name| ActiveValue::Set(Some(name.to_string())))
                .unwrap_or(ActiveValue::Set(None)),
            repeat: argv
                .get("repeat")
                .map(|name| name.iter())
                .into_iter()
                .flatten()
                .into_iter()
                .filter_map(|name| name.parse::<bool>().ok())
                .next()
                .map(|name| ActiveValue::Set(name))
                .unwrap_or(ActiveValue::NotSet),
            state: argv
                .get("state")
                .map(|name| name.iter())
                .into_iter()
                .flatten()
                .into_iter()
                .next()
                .map(|name| ActiveValue::Set(name.to_string()))
                .unwrap_or(ActiveValue::NotSet),
        }
    }
}

impl ToTableRecord<8> for DownloadruleModel {
    fn to_record(&self) -> [Cow<'_, str>; 8] {
        [
            Cow::Owned(self.id.to_string()),
            Cow::Borrowed(&self.name),
            Cow::Owned(
                self.size
                    .map(|size| size.to_string())
                    .unwrap_or("null".to_string()),
            ),
            Cow::Borrowed(
                self.relation_size
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("null"),
            ),
            Cow::Owned(
                self.date
                    .as_ref()
                    .map(NaiveDateTime::to_string)
                    .unwrap_or("null".to_string()),
            ),
            Cow::Borrowed(
                self.relation_date
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("null"),
            ),
            Cow::Owned(self.repeat.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

impl ToTableRecord<8> for &DownloadruleModel {
    fn to_record(&self) -> [Cow<'_, str>; 8] {
        [
            Cow::Owned(self.id.to_string()),
            Cow::Borrowed(&self.name),
            Cow::Owned(
                self.size
                    .map(|size| size.to_string())
                    .unwrap_or("null".to_string()),
            ),
            Cow::Borrowed(
                self.relation_size
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("null"),
            ),
            Cow::Owned(
                self.date
                    .as_ref()
                    .map(NaiveDateTime::to_string)
                    .unwrap_or("null".to_string()),
            ),
            Cow::Borrowed(
                self.relation_date
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("null"),
            ),
            Cow::Owned(self.repeat.to_string()),
            Cow::Borrowed(&self.state),
        ]
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::entity::status_downloadrule::Entity")]
    StatusDownloadrule,
}

impl Related<crate::entity::status::StatusEntity> for Entity {
    fn to() -> RelationDef {
        super::status_downloadrule::Relation::Status.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::status_downloadrule::Relation::Downloadrule
                .def()
                .rev(),
        )
    }
}

// // 关联关系实现（保持与原逻辑一致）
// impl Related<crate::entity::collection::CollectionEntity> for Entity {
//     fn to() -> RelationDef {
//         crate::entity::collection_media::Relation::Set.def()
//     }
//     fn via() -> Option<RelationDef> {
//         Some(crate::entity::collection_media::Relation::Media.def().rev())
//     }
// }

impl ActiveModelBehavior for ActiveModel {}
