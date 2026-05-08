use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Account, Upper};

#[derive(DeriveIden)]
pub enum UpperAccount {
    Table,
    UpperId,
    AccountId,
}
impl UpperAccount {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(UpperAccount::Table)
            .if_not_exists()
            .col(big_unsigned(UpperAccount::UpperId))
            .col(big_unsigned(UpperAccount::AccountId))
            .primary_key(
                Index::create()
                    .col(UpperAccount::UpperId)
                    .col(UpperAccount::AccountId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("upaccount_up_fk")
                    .from(UpperAccount::Table, UpperAccount::UpperId)
                    .to(Upper::Table, Upper::UpperId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("upaccount_account_fk")
                    .from(UpperAccount::Table, UpperAccount::AccountId)
                    .to(Account::Table, Account::AccountId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
