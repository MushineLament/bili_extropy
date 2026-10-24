use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Downloadtask, Media};

#[derive(DeriveIden)]
pub enum DownloadtaskMedias {
    Table,
    TaskId,
    MediaId,
    State,
}

impl DownloadtaskMedias {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(DownloadtaskMedias::Table)
            .if_not_exists()
            .col(big_unsigned(DownloadtaskMedias::TaskId))
            .col(big_unsigned(DownloadtaskMedias::MediaId))
            .col(
                enumeration(
                    DownloadtaskMedias::State,
                    "state",
                    [
                        "Pending",
                        "Downloading",
                        "Completed",
                        "Failed",
                        "Expired",
                        "PermissionDenied",
                        "Unfetch",
                    ],
                )
                .default("Pending"),
            )
            .primary_key(
                Index::create()
                    .col(DownloadtaskMedias::TaskId)
                    .col(DownloadtaskMedias::MediaId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("downloadtask_fk")
                    .from(DownloadtaskMedias::Table, DownloadtaskMedias::TaskId)
                    .to(Downloadtask::Table, Downloadtask::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("media_fk")
                    .from(DownloadtaskMedias::Table, DownloadtaskMedias::MediaId)
                    .to(Media::Table, Media::Aid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
