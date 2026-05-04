use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Media, Upper};

#[derive(DeriveIden)]
pub enum UpperMedia {
    Table,
    MediaId,
    UpperId,
}
impl UpperMedia {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(UpperMedia::Table)
            .if_not_exists()
            .col(big_unsigned(UpperMedia::MediaId))
            .col(big_unsigned(UpperMedia::UpperId))
            .primary_key(
                Index::create()
                    .col(UpperMedia::MediaId)
                    .col(UpperMedia::UpperId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("media_upper_fk")
                    .from(UpperMedia::Table, UpperMedia::MediaId)
                    .to(Media::Table, Media::Aid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("upper_media_fk")
                    .from(UpperMedia::Table, UpperMedia::UpperId)
                    .to(Upper::Table, Upper::UpperId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
