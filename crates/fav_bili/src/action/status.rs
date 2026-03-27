use anyhow::{Context, Ok, Result};

use crate::{
    db::db,
    entity::{ToTableRecord, status::StatusModel},
    state::SetState,
    table::table,
};

pub async fn status() -> Result<()> {
    let db = db(false).await;

    let folders = db.get_active_status().await?;

    let table = table(
        ["Id", "CollectionId", "State", "Name", "Path"],
        [folders].into_iter().map(ToTableRecord::to_record),
    );

    println!("{}\nrows: {}", table, table.count_rows() - 1);

    Ok(())
}

pub async fn status_set(
    name: &str,
    path: &str,
    is_switch: bool,
    collectionid: Option<i64>,
) -> Result<()> {
    let db = db(false).await;

    let state = if is_switch {
        SetState::Active.to_string()
    } else {
        SetState::Inactive.to_string()
    };

    let old_active = db.get_active_status().await;

    let current_id = match db.get_status_by_folder(name, path).await {
        Result::Ok(model) => {
            if let Some(collectionid) = collectionid {
                db.set_status_collection(
                    model.id.context(anyhow::anyhow!(
                        "update status collection id then get entity id error"
                    ))?,
                    collectionid,
                )
                .await?;
            }
            db.activate_status_by_id(
                model
                    .id
                    .context(anyhow::anyhow!("Get status error:{:?}", model))?,
            )
            .await?;
            model.id
        }
        Err(_) => {
            // 插入新路径
            db.upsert_status([StatusModel {
                id: None,
                name: name.to_string(),
                path: path.to_string(),
                collection_id: collectionid,
                state,
            }])
            .await?;
            None
        }
    };

    if let Result::Ok(model) = old_active
        && current_id != model.id
    {
        db.deactivate_status_by_id(
            model
                .id
                .context(anyhow::anyhow!("Get status error:{:?}", model))?,
        )
        .await?;
    }

    Ok(())
}
