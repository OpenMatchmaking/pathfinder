/// The following module contains the simple middleware that
/// don't applying any check to the passed data.
///

use std::collections::HashMap;
use std::sync::Arc;

use futures::future::lazy;

use crate::engine::middleware::base::{Middleware, MiddlewareFuture};
use crate::engine::serializer::JsonMessage;
use crate::rabbitmq::RabbitMQContext;

/// A middleware that used for reverse proxy for cases when
/// not necessary to do validating tokens or permissions.
pub struct EmptyMiddleware;

impl EmptyMiddleware {
    pub fn new() -> EmptyMiddleware {
        EmptyMiddleware {}
    }
}

impl Middleware for EmptyMiddleware {
    /// Returns an empty future which is doesn't doing anything.
    fn process_request(&self, _message: JsonMessage, _rabbitmq_context: Arc<RabbitMQContext>) -> MiddlewareFuture {
        Box::new(lazy(move || Ok(HashMap::new())))
    }
}
