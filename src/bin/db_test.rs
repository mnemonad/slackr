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

    db.setup(&client).await?;

    let user = "U07DL8C7VSQ";
    let channel = "C07R5Q3SGG1";

    let real_name = db.get_real_name(user)?;
    println!("{real_name}");
    
    let channel_name = db.get_channel_from_id(channel)?;
    println!("{channel_name}");

    Ok(())
}
