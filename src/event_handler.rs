use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;
use crate::client::{ SlackEnvelope };

pub type Predicate = fn(&SlackEnvelope) -> bool;

type Callback = Box<dyn Fn(&SlackEnvelope) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;


/// EventHandler manages callbacks for the SlackClient
pub struct EventHandler {
    callbacks: Vec<(Predicate, Callback)>
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new()
        }
    }

    pub fn register_callback<F, Fut>(&mut self, predicate: Predicate, callback: F)
    where
        F: Fn(&SlackEnvelope) -> Fut + Send + Sync + 'static,
        Fut: Future<Output=()> + Send + 'static
    {
        let async_callback: Callback = Box::new(
            move |envelope: &SlackEnvelope| {
                let fut = callback(envelope);
                Box::pin(fut) as Pin<Box<dyn Future<Output=()> + Send>>
            });
        self.callbacks.push((predicate, async_callback));
    }

    pub async fn handle_event(&self, envelope: &SlackEnvelope) {
        for (predicate, callback) in &self.callbacks {
            if predicate(envelope) {
                callback(envelope).await;
            }
        }
    }
}
