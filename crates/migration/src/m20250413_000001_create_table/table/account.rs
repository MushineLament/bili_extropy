use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Account {
    Table,
    AccountId,
    Name,
    Cookies,
    State,
}

impl Account {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Account::Table)
            .if_not_exists()
            .col(big_unsigned_uniq(Account::AccountId))
            .col(string(Account::Name))
            .col(string(Account::Cookies))
            .col(
                enumeration(Account::State, "state", ["Active", "Inactive", "Expired"])
                    .default("Active"),
            )
            .primary_key(Index::create().col(Account::AccountId))
            .to_owned()
    }
}
