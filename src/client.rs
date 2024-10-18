use std::future::Future;
use serde::Deserialize;
use serde_json::json;
use reqwest::{ Client, header::AUTHORIZATION };
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream, tungstenite::Message };
use futures_util::stream::{ SplitSink, SplitStream, StreamExt };
use futures_util::SinkExt;
use crate::event_handler::{EventHandler, Predicate };

fn get_oauth_token() -> String {
    std::env::var("SLACK_OAUTH_TOKEN").expect("ENV ERROR: SLACK_OAUTH_TOKEN")
}

/// Struct to parse WebSocket url request
#[derive(Debug, Deserialize)]
struct AppSocketReponse {
    ok: bool,
    url: Option<String>
}

/// Parse WebSocket Slack Message Envelope
#[derive(Debug, Deserialize)]
pub struct SlackEnvelope {
    envelope_id: String,
    #[serde(rename="type")]
    message_type: String,
    pub payload: SlackPayload
}

/// Slack Message Payload
#[derive(Debug, Deserialize)]
pub struct SlackPayload {
    pub event: Event,
    event_id: String
}

/// Slack Event - Message contents
#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(rename="type")]
    pub event_type: String,
    pub user: String,
    pub text: String,
    pub channel: String
}

/// Slack user list response
#[derive(Debug, Deserialize)]
struct SlackUsersResponse {
    // cache_ts: i64,
    members: Vec<Member>
}

/// Slack User information
#[derive(Debug, Deserialize)]
pub struct Member {
    pub deleted: bool,
    #[serde(rename="id")]
    pub user_id: String,
    pub name: String,
    pub real_name: Option<String>
}

/// Slack channel list response
#[derive(Debug, Deserialize)]
struct SlackChannelsResponse {
    channels: Vec<Channel>
}

/// Slack Channel information
#[derive(Debug, Deserialize)]
pub struct Channel {
    #[serde(rename="id")]
    pub channel_id: String,
    pub name: String,
    pub is_private: bool
}

type WSStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WSWriter = SplitSink<WSStream, Message>;
type WSReader = SplitStream<WSStream>;

#[derive(Debug)]
struct SlackClientStream {
    writer: WSWriter,
    reader: WSReader
} 

/// SlackClient provides methods for both Socket and Web API.
/// Only supports listening + ack over the Socket API.
/// Various methods (more to come) provided for the Web API.
/// Must provide APP token to the connect_to_socket method via literal or through an environment
/// variable: SLACK_APP_TOKEN.
/// For the Web API methods, the oauth token must be provided through an environment variable: SLACK_OAUTH_TOKEN.
pub struct SlackClient {
    pub client: Client,
    stream: Option<SlackClientStream>,
    event_handler: EventHandler
}

// const SLACK_URL: &str = "https://slack.com/api/";

impl SlackClient {
    ///```
    ///let client = Client::new();
    ///```
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            stream: None,
            event_handler: EventHandler::new()
        }
    }

    async fn get_socket_url(&self, api_token: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
        // Allow user to pass app token directly or through environment variable
        let token = match api_token {
            Some(s) => s.to_owned(),
            None => {
                std::env::var("SLACK_APP_TOKEN").expect("ENV ERROR: SLACK_APP_TOKEN")
            }
        };

        let connection_url = "https://slack.com/api/apps.connections.open";
        // Request websocket url from slack api
        let response = self.client.post(connection_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send().await.expect("TODO: Retry");

        // Parse response for access to url
        let response: AppSocketReponse = response.json().await?;
        // Return url if present in the response
        if response.ok {
            let url = response.url.ok_or("URL not found")?;
            Ok(url)
        } else {
            Err("Slack API responed with an error".into())
        }
    }

    ///```
    ///client.connect_to_socket(None).await; // If None provided, .env is used
    ///```
    pub async fn connect_to_socket(&mut self, api_token: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let websocket_url = self.get_socket_url(api_token).await?;

        let (stream, _) = connect_async(websocket_url).await.expect("Failed to connect to socket");
        let (mut writer, mut reader) = stream.split();

        self.stream = Some(SlackClientStream { writer, reader });
        Ok(())
    }

    /// ```
    /// fn contains_no_spaces(event: &SlackEnvelope) -> bool {
    ///     let msg = &event.payload.event.text.clone();
    ///     !msg.contains(' ')
    /// }
    ///
    /// client.register_callback(
    ///     contains_no_spaces,
    ///     move |event: &SlackEnvelope| {
    ///         let message = event.payload.event.text.clone();
    ///         let user = event.payload.event.user.clone();
    ///         let channel = event.payload.event.channel.clone();
    ///         let client = SlackClient::new();  // Unfortunate, but necessary for now
    ///         async move {
    ///             let echo_msg = format!("Received message: {:?} from {:?} in {:?}",
    ///                 &message, &user, &channel);
    ///
    ///             let _ = client.send_message(&channel, &echo_msg).await;
    ///         }
    ///     }
    /// );
    ///```
    pub fn register_callback<F, Fut>(&mut self, predicate: Predicate, callback: F)
    where
        F: Fn(&SlackEnvelope) -> Fut + Send + Sync + 'static,
        Fut: Future<Output=()> + Send + 'static
    {
        self.event_handler.register_callback(predicate, callback);
    }

    async fn ack(&mut self, envelope: &SlackEnvelope) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut stream) = self.stream {
            let ack = json!({
                "envelope_id": envelope.envelope_id,
                "payload": {}
            });
            stream.writer.send(Message::Text(ack.to_string())).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            println!("Ack'd");
            Ok(())
        } else {
            Err("WebSocket stream is not connected".into())
        }
    }

    pub async fn listen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let incoming_event = {
                let stream = self.stream.as_mut().ok_or("WSS is not connected")?;
                stream.reader.next().await
            };

            match incoming_event {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(envelope) = serde_json::from_str::<SlackEnvelope>(&text) {
                        self.ack(&envelope).await?;
                        self.event_handler.handle_event(&envelope).await;
                    }
                },
                Some(Err(e)) => {
                    eprintln!("Error: {:?}", e);
                },
                None => {
                    eprintln!("Connection closed");
                }
                _ => {}
            }
        }
    }

    pub async fn get_user_list(&self) -> Result<Vec<Member>, Box<dyn std::error::Error>> {
        let url = "https://slack.com/api/users.list";
        let response = self.client.post(url)
            .header(AUTHORIZATION, format!("Bearer {}", get_oauth_token()))
            .send().await.expect("Failed to get user list");
        let response = response.json::<SlackUsersResponse>().await?;
        Ok(response.members)
    }
   
    pub async fn get_channel_list(&self) -> Result<Vec<Channel>, Box<dyn std::error::Error>> {
        let url = "https://slack.com/api/conversations.list";
        let response = self.client.post(url)
            .header(AUTHORIZATION, format!("Bearer {}", get_oauth_token()))
            .send().await.expect("Failed to get channel list");
        let response = response.json::<SlackChannelsResponse>().await?;
        Ok(response.channels)
    }

    /* pub async fn test_endpoint(&self, endpoint: &str) {
        todo!()
    }
    */

    pub async fn send_message(&self, channel: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let form = [("channel", channel), ("text", message)];

        let url = "https://slack.com/api/chat.postMessage";
        let response = self.client.post(url)
            .header(AUTHORIZATION, format!("Bearer {}", get_oauth_token()))
            .form(&form)
            .send().await.expect("Failed to send message");

        Ok(())
    }
    
}



