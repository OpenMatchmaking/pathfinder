//! Special kind structures for handling passed options into futures.
//!

use std::sync::Arc;

use futures::sync::mpsc;
use tungstenite::Message;

use super::router::ReadOnlyEndpoint;
use super::serializer::JsonMessage;


/// Simple wrapper for options that will be passed to futures.
pub struct RpcOptions {
    endpoint: ReadOnlyEndpoint,
    message: JsonMessage,
    transmitter: Arc<mpsc::UnboundedSender<Message>>,
    queue_name: Arc<String>
}


impl RpcOptions {
    pub fn new(endpoint: ReadOnlyEndpoint, message: JsonMessage, transmitter: Arc<mpsc::UnboundedSender<Message>>, queue_name: Arc<String>) -> RpcOptions {
        RpcOptions { endpoint, message, transmitter, queue_name }
    }

    pub fn endpoint(&self) -> &ReadOnlyEndpoint {
        &self.endpoint
    }

    pub fn message(&self) -> &JsonMessage {
        &self.message
    }

    pub fn transmitter(&self) -> &Arc<mpsc::UnboundedSender<Message>> {
        &self.transmitter
    }

    pub fn queue_name(&self) -> &Arc<String> {
        &self.queue_name
    }
}
