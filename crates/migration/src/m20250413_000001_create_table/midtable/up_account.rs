use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250413_000001_create_table::{Account, Up};

#[derive(DeriveIden)]
pub enum UpAccount {
    Table,
    UpId,
    AccountId,
}
impl UpAccount {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(UpAccount::Table)
            .if_not_exists()
            .col(big_unsigned(UpAccount::UpId))
            .col(big_unsigned(UpAccount::AccountId))
            .primary_key(
                Index::create()
                    .col(UpAccount::UpId)
                    .col(UpAccount::AccountId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("upaccount_up_fk")
                    .from(UpAccount::Table, UpAccount::UpId)
                    .to(Up::Table, Up::UpId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("upaccount_account_fk")
                    .from(UpAccount::Table, UpAccount::AccountId)
                    .to(Account::Table, Account::AccountId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
