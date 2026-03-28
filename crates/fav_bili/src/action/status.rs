use anyhow::{Context, Ok, Result};

use crate::{
    db::db,
    entity::{ToTableRecord, status::StatusModel},
    state::SetState,
    table::table,
};

pub async fn status() -> Result<()> {
    let db = db(false).await;

    let folders = match db.get_active_status().await {
        Result::Ok(model) => model,
        Err(_) => {
            let test = db.all_status().await?;
            if !test.is_empty() {
                return Err(anyhow::anyhow!("Not anyone status is active"));
            }

            db.upsert_status([StatusModel {
                id: None,
                name: "".to_owned(),
                path: ".".to_owned(),
                state: SetState::Active.to_string(),
            }])
            .await
            .context("add a default status folder path error")?;

            db.get_active_status()
                .await
                .context("get a default status folder path error")?
        }
    };

    let table = table(
        ["Id", "State", "Name", "Path"],
        folders.into_iter().map(ToTableRecord::to_record),
    );

    println!("{}\nrows: {}", table, table.count_rows() - 1);

    Ok(())
}

pub async fn status_set(name: &str, path: &str, is_switch: bool) -> Result<()> {
    let db = db(false).await;

    let state = if is_switch {
        SetState::Active.to_string()
    } else {
        SetState::Inactive.to_string()
    };

    let old_active = db.get_active_status().await;

    let current_id = match db.get_status_by_folder(name, path).await {
        Result::Ok(model) => {
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
                state,
            }])
            .await?;
            None
        }
    };

    if let Result::Ok(models) = old_active {
        for model in models {
            if current_id == model.id {
                continue;
            }

            db.deactivate_status_by_id(
                model
                    .id
                    .context(anyhow::anyhow!("Get status error:{:?}", model))?,
            )
            .await?;
        }
    }

    Ok(())
}
