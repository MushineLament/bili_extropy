use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Media {
    Table,
    /// 视频的Aid
    Aid,
    /// 视频的BvId
    BvId,
    /// 视频up主的Cid
    Cid,
    /// 视频标题
    Title,
    /// 视频的类型(?)
    Type,
}

impl Media {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Media::Table)
            .if_not_exists()
            .col(big_unsigned_uniq(Media::Aid))
            .col(string_uniq(Media::BvId))
            .col(big_unsigned(Media::Cid))
            .col(string(Media::Title))
            .col(
                enumeration(Media::Type, "type", ["Video", "Audio", "Collection"]).default("Video"),
            )
            .primary_key(Index::create().col(Media::Aid))
            .to_owned()
    }
}
