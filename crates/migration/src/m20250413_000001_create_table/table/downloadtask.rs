use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Downloadtask {
    Table,
    Id,
    TypeId,
    GenericId,
    State,
}

impl Downloadtask {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Downloadtask::Table)
            .if_not_exists()
            .col(
                big_unsigned(Downloadtask::Id)
                    .auto_increment()
                    .primary_key(),
            )
            .col(
                enumeration(
                    Downloadtask::State,
                    "state",
                    [
                        "Pending",
                        "Downloading",
                        "Completed",
                        "Failed",
                        "Expired",
                        "PermissionDenied",
                        "Unfetch",
                    ],
                )
                .default("Pending"),
            )
            .col(tiny_unsigned(Downloadtask::TypeId).not_null())
            .col(big_unsigned(Downloadtask::GenericId).not_null())
            .to_owned()
    }
}
