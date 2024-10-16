use serde::Deserialize;
use serde_json::json;
use reqwest::{ Client, header::AUTHORIZATION };
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream, tungstenite::Message };
use futures_util::stream::{ SplitSink, SplitStream, StreamExt };
use futures_util::SinkExt;
use futures::future::BoxFuture;
use crate::event_handler::EventHandler;

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
    pub(crate) event_type: String,
    pub user: String,
    pub text: String,
    pub channel: String
}

/// Slack user list response
#[derive(Debug, Deserialize)]
struct SlackUsersResponse {
    cache_ts: i64,
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

// #[derive(Debug)]
pub struct SlackClient {
    client: Client,
    stream: Option<SlackClientStream>,
    event_handler: EventHandler
}

const SLACK_URL: &str = "https://slack.com/api/";

impl SlackClient {
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

    pub async fn connect_to_socket(&mut self, api_token: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let websocket_url = self.get_socket_url(api_token).await?;

        let (stream, _) = connect_async(websocket_url).await.expect("Failed to connect to socket");
        let (mut writer, mut reader) = stream.split();

        self.stream = Some(SlackClientStream { writer, reader });
        Ok(())
    }

    pub fn register_callback<F>(&mut self, event_type: &str, callback: F)
        where
            F: Fn(SlackEnvelope) -> BoxFuture<'static, ()> + 'static
    {
        self.event_handler.register_callback(event_type, callback);
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
            let message = {
                let stream = self.stream.as_mut().ok_or("WSS is not connected")?;
                stream.reader.next().await
            };

            match message {
                Some(Ok(msg)) => {
                    if let Ok(text) = msg.to_text() {
                        if let Ok(parsed) = serde_json::from_str::<SlackEnvelope>(text) {

                            self.ack(&parsed).await?;
                            self.event_handler.handle_event(parsed).await;

                        }
                    }
                },
                Some(Err(e)) => eprintln!("ERROR: {}", e),
                None => {
                    eprintln!("Stream ended");
                }
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

    pub async fn test_endpoint(&self, endpoint: &str) {
        todo!()
    }

    pub async fn send_message(&self, channel: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let form = [("channel", channel), ("text", message)];

        let url = "https://slack.com/api/chat.postMessage";
        let response = self.client.post(url)
            .header(AUTHORIZATION, format!("Bearer {}", get_oauth_token()))
            .form(&form)
            .send().await.expect("Failed to send message");

        dbg!(response.text().await?);
        Ok(())
    }
    
}



