/// The following module contains the simple middleware that
/// don't applying any check to the passed data.
///

use std::collections::HashMap;
use std::sync::Arc;

use futures::future::lazy;

use engine::middleware::base::{Middleware, MiddlewareFuture};
use engine::serializer::JsonMessage;
use rabbitmq::RabbitMQClient;

/// Default struct which is used for reverse proxy without an authentication
/// layer.
pub struct EmptyMiddleware;

impl EmptyMiddleware {
    pub fn new() -> EmptyMiddleware {
        EmptyMiddleware {}
    }
}

impl Middleware for EmptyMiddleware {
    /// Returns an empty future which is doesn't doing anything.
    fn process_request(&self, _message: JsonMessage, _rabbitmq_client: Arc<RabbitMQClient>) -> MiddlewareFuture {
        Box::new(lazy(move || Ok(HashMap::new())))
    }
}
