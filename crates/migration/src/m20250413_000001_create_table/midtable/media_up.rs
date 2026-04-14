use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Media, Up};

#[derive(DeriveIden)]
pub enum MediaUp {
    Table,
    Id,
    UpId,
}
impl MediaUp {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(MediaUp::Table)
            .if_not_exists()
            .col(big_unsigned(MediaUp::Id))
            .col(big_unsigned(MediaUp::UpId))
            .primary_key(Index::create().col(MediaUp::Id).col(MediaUp::UpId))
            .foreign_key(
                ForeignKey::create()
                    .name("mediaup_media_fk")
                    .from(MediaUp::Table, MediaUp::Id)
                    .to(Media::Table, Media::Aid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("mediaup_up_fk")
                    .from(MediaUp::Table, MediaUp::UpId)
                    .to(Up::Table, Up::UpId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
