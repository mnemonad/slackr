use std::path::Path;
use slackr::alias_db::Database;
use slackr::client::SlackClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = env!("CARGO_MANIFEST_DIR");
    let db_path = Path::new(dir).join("assets/alias_db.db");
    let db = Database::new(&db_path)?;

    let dotenv_path = Path::new(dir).join("assets/.env");
    dotenv::from_path(dotenv_path).ok();
    let client = SlackClient::new();

    let members = client.get_user_list().await?;
    db.insert_members(members)?;

    let channels = client.get_channel_list().await?;
    db.insert_channels(channels)?;

    Ok(())
}
