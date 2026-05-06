//! `SeaORM` Entity, 已手动转换为 DeriveEntityModel 现代化写法

use std::borrow::Cow;

use sea_orm::entity::prelude::*;
use serde::Deserialize;
use url::Url;

use crate::{entity::UpperCid, table::ToTableRecord};

pub const UPPER: &str = "Upper";

// ========== 向后兼容别名 ==========
pub use Entity as UpperEntity;
pub use Model as UpperModel;

// ========== 实体模型 ==========

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "upper")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub upper_id: UpperCid,
    pub name: String,
}

// ========== 关系定义 ==========

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "crate::entity::upper_media::Entity")]
    MediaUp,
    #[sea_orm(has_many = "crate::entity::upper_account::Entity")]
    UpAccount,
}

impl Related<crate::entity::upper_media::UpperMediaEntity> for Entity {
    fn to() -> RelationDef {
        Relation::MediaUp.def()
    }
}

impl Related<crate::entity::upper_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UpAccount.def()
    }
}

impl Related<crate::entity::account::AccountEntity> for Entity {
    fn to() -> RelationDef {
        crate::entity::upper_account::Relation::Account.def()
    }
    fn via() -> Option<RelationDef> {
        Some(crate::entity::upper_account::Relation::Up.def().rev())
    }
}

impl Related<crate::entity::media::MediaEntity> for Entity {
    fn to() -> RelationDef {
        crate::entity::upper_media::Relation::Media.def()
    }
    fn via() -> Option<RelationDef> {
        Some(crate::entity::upper_media::Relation::Upper.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

// ========== 表格显示 trait 实现 ==========

impl ToTableRecord<2> for Model {
    fn to_record(&self) -> [Cow<'_, str>; 2] {
        [
            Cow::Owned(self.upper_id.to_string()),
            Cow::Borrowed(&self.name),
        ]
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Upper {
    pub mid: i64,
    #[serde(alias = "uname")]
    pub name: String,
    pub face: Url,
}

#[derive(Debug, Deserialize)]
pub struct FollowingUpResp {
    pub data: FollowingUpData,
}

#[derive(Debug, Deserialize)]
pub struct FollowingUpData {
    pub list: Vec<Upper>,
}

#[derive(Debug, Deserialize)]
pub struct FollowingNumResp {
    pub data: FollowingNumData,
}

#[derive(Debug, Deserialize)]
pub struct FollowingNumData {
    pub following: i64,
}

#[derive(Debug, Deserialize)]
pub struct PublishNumResp {
    pub data: PublishNumData,
}

#[derive(Debug, Deserialize)]
pub struct PublishNumData {
    pub video: i64,
}

use api_req::{Method, Payload};
use serde::Serialize;

#[derive(Debug, Payload, Serialize)]
#[api_req(path = "/x/passport-login/web/qrcode/generate")]
pub struct QrPayload;

#[derive(Debug, Payload, Serialize)]
#[api_req(path = "/x/passport-login/web/qrcode/poll")]
pub struct QrPollPayload {
    pub qrcode_key: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Payload, Serialize)]
#[api_req(path = "/login/exit/v2", method = Method::POST, req = form)]
pub struct LogoutPayload {
    pub biliCSRF: String,
}
