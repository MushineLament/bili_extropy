use anyhow::Result;
use futures::TryFutureExt;
use sea_orm::{
    ActiveValue::{Set, Unchanged},
    ConnectionTrait, DatabaseBackend, EntityTrait as _, IntoActiveModel as _, Statement,
    sea_query::OnConflict,
};

use super::Db;
use crate::{entity::media, state::MediaState};

impl Db {
    pub async fn upsert_medias(
        &self,
        medias: impl IntoIterator<Item = media::MediaModel>,
    ) -> Result<()> {
        media::MediaEntity::insert_many(medias.into_iter().map(|m| m.into_active_model()))
            .on_conflict(
                OnConflict::column(media::Column::Aid)
                    .update_columns([
                        media::Column::BvId,
                        media::Column::Cid,
                        media::Column::Title,
                        media::Column::Type,
                        media::Column::State,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&self.db)
            .await?;
        Ok(())
    }

    pub async fn set_media_state(&self, id: i64, state: MediaState) -> Result<()> {
        media::MediaEntity::update(media::ActiveModel {
            aid: Unchanged(id),
            state: Set(state.to_string()),
            ..Default::default()
        })
        .exec(&self.db)
        .map_err(|err| {
            anyhow::anyhow!(
                "set media state err: {:?},caller: {:?}",
                err,
                (file!(), line!())
            )
        })
        .await?;
        Ok(())
    }

    pub async fn all_medias(&self) -> Result<Vec<media::MediaModel>> {
        media::MediaEntity::find()
            .all(&self.db)
            .await
            .map_err(Into::into)
    }

    pub async fn all_active_medias(&self) -> Result<Vec<media::MediaModel>> {
        media::MediaEntity::find()
            .from_raw_sql(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
SELECT DISTINCT m.*
FROM media m
WHERE
    EXISTS (
        SELECT 1
        FROM media_up mu
        JOIN up u ON mu.up_id = u.up_id
        WHERE mu.id = m.aid AND u.state = 'Active'
    )
    OR EXISTS (
        SELECT 1
        FROM collection_media cm
        JOIN "collection" s ON cm.collection_id = s.collection_id
        WHERE cm.id = m.aid AND s.state = 'Active'
    );
"#,
            ))
            .all(&self.db)
            .await
            .map_err(Into::into)
    }

    pub async fn all_active_pending_medias(&self) -> Result<Vec<media::MediaModel>> {
        media::MediaEntity::find()
            .from_raw_sql(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
SELECT DISTINCT m.*
FROM media m
WHERE
m.state = 'Pending'
AND (
    EXISTS (
        SELECT 1
        FROM media_up mu
        JOIN up u ON mu.up_id = u.up_id
        WHERE mu.id = m.aid AND u.state = 'Active'
    )
    OR EXISTS (
        SELECT 1
        FROM collection_media ms
        JOIN "collection" s ON ms.collection_id = s.collection_id
        WHERE ms.id = m.aid AND s.state = 'Active'
    )
);
"#,
            ))
            .all(&self.db)
            .await
            .map_err(Into::into)
    }

    /// Cleanup the medias whose up and set both are inactive/null
    pub async fn prune_medias(&self) -> Result<()> {
        self.db
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                r#"
DELETE FROM media
WHERE id IN (
    SELECT m.id
    FROM media m
    WHERE NOT EXISTS (
        SELECT 1 FROM media_up mu
        JOIN up u ON mu.up_id = u.up_id
        WHERE mu.id = m.id AND u.state != 'Inactive'
    )
    AND NOT EXISTS (
        SELECT 1 FROM media_set ms
        JOIN "set" s ON ms.collection_id = s.collection_id
        WHERE ms.id = m.id AND s.state != 'Inactive'
    )
);
"#,
            ))
            .await?;
        Ok(())
    }
}
