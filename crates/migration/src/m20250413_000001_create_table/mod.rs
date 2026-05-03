#![allow(clippy::enum_variant_names)]

use sea_orm_migration::prelude::*;

mod table;
pub use table::*;

mod midtable;
pub use midtable::*;

mod detailtable;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. 基表（不依赖其他表）
        manager.create_table(Media::create_table()).await?;
        manager.create_table(Status::create_table()).await?;
        manager.create_table(Account::create_table()).await?;
        manager.create_table(Upper::create_table()).await?;
        manager.create_table(Collection::create_table()).await?;
        manager.create_table(Downloadrule::create_table()).await?;
        manager.create_table(DownloadTask::create_table()).await?;

        // 2. 中间表（依赖基表）
        manager.create_table(StatusMedia::create_table()).await?;
        manager
            .create_table(StatusCollection::create_table())
            .await?;
        manager
            .create_table(CollectionMedia::create_table())
            .await?;
        manager.create_table(UpperMedia::create_table()).await?;
        manager
            .create_table(AccountCollection::create_table())
            .await?;
        manager.create_table(UpperAccount::create_table()).await?;
        manager
            .create_table(StatusDownloadrule::create_table())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 先删除中间表
        manager
            .drop_table(Table::drop().table(UpperAccount::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AccountCollection::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UpperMedia::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CollectionMedia::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(StatusMedia::Table).to_owned())
            .await?; // 添加这一行

        manager
            .drop_table(Table::drop().table(StatusCollection::Table).to_owned())
            .await?; // 添加这一行

        manager
            .drop_table(Table::drop().table(StatusDownloadrule::Table).to_owned())
            .await?;

        // 再删除基表
        manager
            .drop_table(Table::drop().table(Media::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Collection::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Upper::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Account::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Status::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(DownloadTask::Table).to_owned())
            .await?;

        Ok(())
    }
}
