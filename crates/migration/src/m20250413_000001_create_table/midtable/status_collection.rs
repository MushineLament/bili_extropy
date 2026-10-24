use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Collection, Status};

#[derive(DeriveIden)]
pub enum StatusCollection {
    Table,
    StatusId,
    CollectionId,
}

impl StatusCollection {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Self::Table)
            .if_not_exists()
            .col(big_unsigned(Self::CollectionId))
            .col(big_unsigned(Self::StatusId))
            .primary_key(Index::create().col(Self::CollectionId).col(Self::StatusId))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_statuscollection_collection")
                    .from(Self::Table, Self::CollectionId)
                    .to(Collection::Table, Collection::CollectionId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_statuscollection_status")
                    .from(Self::Table, Self::StatusId)
                    .to(Status::Table, Status::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
