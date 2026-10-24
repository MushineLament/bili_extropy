use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Media, Status};

#[derive(DeriveIden)]
pub enum StatusMedia {
    Table,
    StatusId,
    MediaAid,
}

impl StatusMedia {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(StatusMedia::Table)
            .if_not_exists()
            .col(big_unsigned(StatusMedia::MediaAid))
            .col(big_unsigned(StatusMedia::StatusId))
            .primary_key(
                Index::create()
                    .col(StatusMedia::MediaAid)
                    .col(StatusMedia::StatusId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_statusmedia_media")
                    .from(StatusMedia::Table, StatusMedia::MediaAid)
                    .to(Media::Table, Media::Aid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_statusmedia_status")
                    .from(StatusMedia::Table, StatusMedia::StatusId)
                    .to(Status::Table, Status::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
