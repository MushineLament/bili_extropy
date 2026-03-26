use anyhow::{Ok, Result};

use crate::{db::db, entity::status::StatusModel, state::SetState};

pub async fn status() -> Result<()> {
    let db = db(false).await;

    let folder = match db.get_status().await {
        Result::Ok(folder) => folder,
        Err(err) => {
            println!("err:{:?}", err);

            return Err(err);
        }
    };

    println!("id:{:?}", folder.id);
    println!("name:{:?}", folder.name);
    println!("path:{:?}", folder.path);
    println!("collection_id:{:?}", folder.collection_id);
    println!("state:{:?}", folder.state);

    Ok(())
}

pub async fn status_set(name: &str, path: &str) -> Result<()> {
    let db = db(false).await;
    db.upsert_status([StatusModel {
        id: 0,
        name: name.to_string(),
        path: path.to_string(),
        collection_id: None,
        state: SetState::Unreachable.to_string(),
    }])
    .await
}
