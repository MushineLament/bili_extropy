use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Upper {
    Table,
    UpperId,
    Name,
}

impl Upper {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Upper::Table)
            .if_not_exists()
            .col(big_unsigned_uniq(Upper::UpperId))
            .col(string(Upper::Name))
            .primary_key(Index::create().col(Upper::UpperId))
            .to_owned()
    }
}
