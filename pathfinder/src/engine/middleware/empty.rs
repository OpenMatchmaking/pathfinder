/// The following module contains the simple middleware that
/// don't applying any check to the passed data.
///

use futures::future::lazy;

use engine::middleware::base::{Middleware, MiddlewareFuture};
use engine::serializer::JsonMessage;

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
    fn process_request(&self, _message: JsonMessage) -> MiddlewareFuture {
        Box::new(lazy(move || Ok(())))
    }
}
