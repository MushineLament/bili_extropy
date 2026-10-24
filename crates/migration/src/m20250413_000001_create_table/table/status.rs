use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveIden)]
pub enum Status {
    Table,
    /// Status的自增Id，用来指定下载的Id
    Id,
    /// 下载到的文件名中
    Name,
    /// 下载路径
    Path,
    /// 是否激活状态
    State,
}

impl Status {
    pub fn create_table() -> TableCreateStatement {
        Table::create()
            .table(Status::Table)
            .if_not_exists()
            .col(big_unsigned(Status::Id).auto_increment().primary_key())
            .col(string(Status::Name).default("."))
            .col(string(Status::Path).default("."))
            .col(
                enumeration(
                    Status::State,
                    "state",
                    ["Active", "Inactive", "Unreachable"],
                )
                .default("Inactive"),
            )
            .to_owned()
    }
}
