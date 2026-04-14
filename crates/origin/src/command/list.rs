// use anyhow::Result;

// pub async fn list_medias() -> Result<()> {
//     let db = db(false).await;
//     let medias = db.all_medias().await?;
//     let table = table(
//         ["id", "bvid", "title", "type", "state"],
//         medias.into_iter().map(ToTableRecord::to_record),
//     );
//     println!("{}\nrows: {}", table, table.count_rows() - 1);
//     Ok(())
// }
