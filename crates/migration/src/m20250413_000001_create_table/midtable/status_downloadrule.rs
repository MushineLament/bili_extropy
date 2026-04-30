use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{DownloadRule, Status};

#[derive(DeriveIden)]
pub enum StatusDonwloadRule {
    Table,
    StatusId,
    RuleId,
}
impl StatusDonwloadRule {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(StatusDonwloadRule::Table)
            .if_not_exists()
            .col(big_unsigned(StatusDonwloadRule::StatusId))
            .col(big_unsigned(StatusDonwloadRule::RuleId))
            .primary_key(
                Index::create()
                    .col(StatusDonwloadRule::StatusId)
                    .col(StatusDonwloadRule::RuleId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("status_downloadrule_fk")
                    .from(StatusDonwloadRule::Table, StatusDonwloadRule::StatusId)
                    .to(Status::Table, Status::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("downloadrule_status_fk")
                    .from(StatusDonwloadRule::Table, StatusDonwloadRule::RuleId)
                    .to(DownloadRule::Table, DownloadRule::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
