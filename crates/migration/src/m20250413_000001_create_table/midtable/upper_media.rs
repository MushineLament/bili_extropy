use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum UpperMedia {
    Table,
    MediaId,
    UpperId,
}
impl UpperMedia {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(UpperMedia::Table)
            .if_not_exists()
            .col(big_unsigned(UpperMedia::MediaId))
            .col(big_unsigned(UpperMedia::UpperId))
            .primary_key(
                Index::create()
                    .col(UpperMedia::MediaId)
                    .col(UpperMedia::UpperId),
            )
            .to_owned()
    }
}
