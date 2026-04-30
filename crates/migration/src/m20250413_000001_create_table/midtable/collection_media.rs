use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Collection, Media};

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
            .foreign_key(
                ForeignKey::create()
                    .name("mediaset_media_fk")
                    .from(CollectionMedia::Table, CollectionMedia::MediaCid)
                    .to(Media::Table, Media::Aid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("mediaset_set_fk")
                    .from(CollectionMedia::Table, CollectionMedia::CollectionId)
                    .to(Collection::Table, Collection::CollectionId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
