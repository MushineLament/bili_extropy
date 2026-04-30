use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Media, Upper};

#[derive(DeriveIden)]
pub enum MediaUpper {
    Table,
    Id,
    UpperId,
}
impl MediaUpper {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(MediaUpper::Table)
            .if_not_exists()
            .col(big_unsigned(MediaUpper::Id))
            .col(big_unsigned(MediaUpper::UpperId))
            .primary_key(Index::create().col(MediaUpper::Id).col(MediaUpper::UpperId))
            .foreign_key(
                ForeignKey::create()
                    .name("mediaup_media_fk")
                    .from(MediaUpper::Table, MediaUpper::Id)
                    .to(Media::Table, Media::Aid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("mediaup_up_fk")
                    .from(MediaUpper::Table, MediaUpper::UpperId)
                    .to(Upper::Table, Upper::UpperId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
