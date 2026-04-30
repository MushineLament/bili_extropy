use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum DownloadTask {
    Table,
    TypeId,
    GenericId,
    State,
}

impl DownloadTask {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(DownloadTask::Table)
            .if_not_exists()
            .col(
                enumeration(
                    DownloadTask::State,
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
            .col(tiny_unsigned(DownloadTask::TypeId).not_null())
            .col(big_unsigned_uniq(DownloadTask::GenericId).not_null())
            .primary_key(
                Index::create()
                    .col(DownloadTask::TypeId)
                    .col(DownloadTask::GenericId),
            )
            .to_owned()
    }
}
