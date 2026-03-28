use sea_orm_migration::{prelude::*, schema::*};

use crate::m20250527_000001_create_table::{Account, Collection};

#[derive(DeriveIden)]
pub enum AccountCollection {
    Table,
    SetId,
    AccountId,
}
impl AccountCollection {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(AccountCollection::Table)
            .if_not_exists()
            .col(big_unsigned(AccountCollection::SetId))
            .col(big_unsigned(AccountCollection::AccountId))
            .primary_key(
                Index::create()
                    .col(AccountCollection::SetId)
                    .col(AccountCollection::AccountId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("setaccount_set_fk")
                    .from(AccountCollection::Table, AccountCollection::SetId)
                    .to(Collection::Table, Collection::CollectionId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("setaccount_account_fk")
                    .from(AccountCollection::Table, AccountCollection::AccountId)
                    .to(Account::Table, Account::AccountId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned()
    }
}
