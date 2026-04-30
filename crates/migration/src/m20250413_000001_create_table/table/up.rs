use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Upper {
    Table,
    UpperId,
    Name,
    State,
}
impl Upper {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Upper::Table)
            .if_not_exists()
            .col(big_unsigned_uniq(Upper::UpperId))
            .col(string(Upper::Name))
            .col(
                enumeration(Upper::State, "state", ["Active", "Inactive", "Deactivated"])
                    .default("Inactive"),
            )
            .primary_key(Index::create().col(Upper::UpperId))
            .to_owned()
    }
}
