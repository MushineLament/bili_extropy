use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Up {
    Table,
    UpId,
    Name,
    State,
}
impl Up {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Up::Table)
            .if_not_exists()
            .col(big_unsigned_uniq(Up::UpId))
            .col(string(Up::Name))
            .col(
                enumeration(Up::State, "state", ["Active", "Inactive", "Deactivated"])
                    .default("Inactive"),
            )
            .primary_key(Index::create().col(Up::UpId))
            .to_owned()
    }
}
