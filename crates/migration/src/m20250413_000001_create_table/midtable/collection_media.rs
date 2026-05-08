use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum CollectionMedia {
    Table,
    MediaCid,
    CollectionId,
}

impl CollectionMedia {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(CollectionMedia::Table)
            .if_not_exists()
            .col(big_unsigned(CollectionMedia::MediaCid))
            .col(big_unsigned(CollectionMedia::CollectionId))
            .primary_key(
                Index::create()
                    .col(CollectionMedia::MediaCid)
                    .col(CollectionMedia::CollectionId),
            )
            .to_owned()
    }
}
