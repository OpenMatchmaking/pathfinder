//! Special kind structures for handling passed options into futures.
//!

use std::sync::Arc;

use super::router::ReadOnlyEndpoint;
use super::serializer::JsonMessage;


/// Simple wrapper for options that will be passed to futures.
#[derive(Clone, Debug)]
pub struct RpcOptions {
    endpoint: Option<ReadOnlyEndpoint>,
    message: Option<JsonMessage>,
    queue_name: Option<Arc<String>>
}

impl Default for RpcOptions {
    fn default() -> RpcOptions {
        RpcOptions {
            endpoint: None,
            message: None,
            queue_name: None,
        }
    }
}

impl RpcOptions {
    pub fn with_endpoint(mut self, value: ReadOnlyEndpoint) -> RpcOptions {
        self.endpoint = Some(value);
        self
    }

    pub fn with_message(mut self, value: JsonMessage) -> RpcOptions {
        self.message = Some(value);
        self
    }

    pub fn with_queue_name(mut self, value: Arc<String>) -> RpcOptions {
        self.queue_name = Some(value);
        self
    }

    pub fn get_endpoint(&self) -> Option<ReadOnlyEndpoint> {
        self.endpoint.clone()
    }

    pub fn get_message(&self) -> Option<JsonMessage> {
        self.message.clone()
    }

    pub fn get_queue_name(&self) -> Option<Arc<String>> {
        self.queue_name.clone()
    }
}
