use std::env;
use std::path::Path;
use slackr::client::{ SlackClient, SlackEnvelope };

use futures_util::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = env!("CARGO_MANIFEST_DIR");
    let dotenv_path = Path::new(dir).join("assets/.env");
    dotenv::from_path(dotenv_path.as_path()).ok();

    let mut client = SlackClient::new();

//    let _ = client.send_message("bot-testing", "test message").await?;

    let _ = client.connect_to_socket(None).await;

    client.register_callback("message", |event: SlackEnvelope| {
        Box::pin(async move {
            let msg_event = &event.payload.event;
            println!("Received message: {:?} from {:?} in {:?}", msg_event.text, msg_event.user, msg_event.channel);
        })
    });

    client.listen().await;

    Ok(())
}
