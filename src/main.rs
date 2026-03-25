use fav_bili::command::FavCommand;

use tracing::error;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = FavCommand::new().run().await {
        error!("{}", e);
    }
}
