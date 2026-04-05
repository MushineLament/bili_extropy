use anyhow::Result;
use sea_orm::{ColumnTrait as _, EntityTrait as _, IntoActiveModel as _, QueryFilter as _};

use super::Db;
use crate::entity::account_collection;

impl Db {
    pub async fn upsert_set_accounts(
        &self,
        set_accounts: impl IntoIterator<Item = account_collection::AccountCollectionModel>,
    ) -> Result<()> {
        account_collection::AccountCollectionEntity::insert_many(
            set_accounts.into_iter().map(|m| m.into_active_model()),
        )
        .on_conflict_do_nothing()
        .exec_without_returning(&self.db)
        .await?;
        Ok(())
    }

    pub async fn get_set_ids_of_account(&self, account_id: i64) -> Result<Vec<i64>> {
        account_collection::AccountCollectionEntity::find()
            .filter(account_collection::Column::AccountId.eq(account_id))
            .all(&self.db)
            .await
            .map_err(Into::into)
            .map(|res| res.into_iter().map(|m| m.collection_id).collect())
    }

    pub async fn delete_set_account(
        &self,
        set_account: account_collection::AccountCollectionModel,
    ) -> Result<()> {
        account_collection::AccountCollectionEntity::delete_by_id((
            set_account.collection_id,
            set_account.account_id,
        ))
        .exec(&self.db)
        .await?;
        Ok(())
    }
}
