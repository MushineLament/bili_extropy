use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Collection {
    Table,
    /// 收藏夹id
    CollectionId,
    /// 收藏夹名
    Name,
    /// 收藏夹视频数量
    Count,
    /// 状态
    State,
}

impl Collection {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Collection::Table)
            .if_not_exists()
            .col(big_unsigned_uniq(Collection::CollectionId))
            .col(string(Collection::Name))
            .col(big_unsigned(Collection::Count))
            .col(
                enumeration(
                    Collection::State,
                    "state",
                    ["Active", "Inactive", "Unreachable"],
                )
                .default("Inactive"),
            )
            .primary_key(Index::create().col(Collection::CollectionId))
            .to_owned()
    }
}
