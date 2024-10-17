use std::env;
use std::path::Path;
use slackr::client::{ SlackClient, SlackEnvelope };

fn contains_spaces(event: &SlackEnvelope) -> bool {
    let msg = &event.payload.event.text.clone();
    
    // If you try to run this without innoculating your bot's userID,
    // you will enter a spiral of the bot responding to itself
    let user = &event.payload.event.user.clone();

    let bot_id = "alpha-numeric";
    if user != bot_id {
        return msg.contains(' ');
    }
    false
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = env!("CARGO_MANIFEST_DIR");
    let dotenv_path = Path::new(dir).join("assets/.env");
    dotenv::from_path(dotenv_path.as_path()).ok();

    let mut client = SlackClient::new();

    client.connect_to_socket(None).await;

    client.register_callback(
        contains_spaces,
        move |event: &SlackEnvelope| {
            let message = event.payload.event.text.clone();
            let user = event.payload.event.user.clone();
            let channel = event.payload.event.channel.clone();
            let client = SlackClient::new();  // TODO: Make thread-safe so client is reusable
            async move {
                let echo_msg = format!("Received message: {:?} from {:?} in {:?}",
                    &message, &user, &channel);

                let _ = client.send_message(&channel, &echo_msg).await;
            }
        }
    );

    // client.register_callback("message", |event: SlackEnvelope| {
    //     Box::pin(async move {
    //         let msg_event = &event.payload.event;
    //         println!("Received message: {:?} from {:?} in {:?}", msg_event.text, msg_event.user, msg_event.channel);
    //     })
    // });

    client.listen().await;

    Ok(())
}
