use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum DownloadRule {
    Table,
    Id,
    Name,
    Size,
    Date,
    Repeat,
    State,
}

impl DownloadRule {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(DownloadRule::Table)
            .if_not_exists()
            .col(big_unsigned(DownloadRule::Id))
            .col(string(DownloadRule::Name))
            .col(big_unsigned(DownloadRule::Size))
            .col(date_time(DownloadRule::Date))
            .col(boolean(DownloadRule::Repeat))
            .col(
                enumeration(
                    DownloadRule::State,
                    "state",
                    ["Active", "Inactive", "Expired"],
                )
                .default("Active"),
            )
            .primary_key(Index::create().col(DownloadRule::Id))
            .to_owned()
    }
}
