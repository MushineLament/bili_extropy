use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Downloadrule {
    Table,
    Id,
    Name,
    Size,
    RelationSize,
    Date,
    RelationDate,
    Repeat,
    State,
}

impl Downloadrule {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Downloadrule::Table)
            .if_not_exists()
            .col(
                big_unsigned(Downloadrule::Id)
                    .not_null()
                    .auto_increment()
                    .primary_key(),
            )
            .col(string(Downloadrule::Name))
            .col(ColumnDef::new(Downloadrule::Size).big_unsigned().null())
            .col(
                ColumnDef::new(Downloadrule::RelationSize)
                    .enumeration("relation_size", ["<=", "<", "==", ">", ">="])
                    .take()
                    .null(),
            )
            .col(ColumnDef::new(Downloadrule::Date).date_time().null())
            .col(
                ColumnDef::new(Downloadrule::RelationDate)
                    .enumeration("relation_date", ["<=", "<", "==", ">", ">="])
                    .take()
                    .null(),
            )
            .col(boolean(Downloadrule::Repeat).default(true))
            .col(
                enumeration(
                    Downloadrule::State,
                    "state",
                    ["Active", "Inactive", "Expired"],
                )
                .default("Inactive"),
            )
            .to_owned()
    }
}
