use std::collections::HashMap;
use futures::future::BoxFuture;
use crate::client::{ SlackEnvelope };

type Callback = Box<dyn Fn(SlackEnvelope) -> BoxFuture<'static, ()>>;

/// EventHandler manages callbacks for the SlackClient
pub struct EventHandler {
    callbacks: HashMap<String, Callback>
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new()
        }
    }

    pub fn register_callback<F>(&mut self, event_type: &str, callback: F)
        where
            F: Fn(SlackEnvelope) -> BoxFuture<'static, ()> + 'static
    {
        self.callbacks.insert(event_type.to_owned(), Box::new(callback));
    }

    pub async fn handle_event(&self, envelope: SlackEnvelope) {
        if let Some(callback) = self.callbacks.get(&envelope.payload.event.event_type) {
            callback(envelope).await;
        }
    }
}
