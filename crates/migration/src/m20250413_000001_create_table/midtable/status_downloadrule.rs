use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Downloadrule, Status};

#[derive(DeriveIden)]
pub enum StatusDownloadrule {
    Table,
    StatusId,
    RuleId,
}
impl StatusDownloadrule {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(StatusDownloadrule::Table)
            .if_not_exists()
            .col(big_unsigned(StatusDownloadrule::StatusId))
            .col(big_unsigned(StatusDownloadrule::RuleId))
            .primary_key(
                Index::create()
                    .col(StatusDownloadrule::StatusId)
                    .col(StatusDownloadrule::RuleId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("status_downloadrule_fk")
                    .from(StatusDownloadrule::Table, StatusDownloadrule::StatusId)
                    .to(Status::Table, Status::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("downloadrule_status_fk")
                    .from(StatusDownloadrule::Table, StatusDownloadrule::RuleId)
                    .to(Downloadrule::Table, Downloadrule::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
