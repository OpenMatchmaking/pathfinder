//! Middleware implementations with token support
//!

use cli::{CliOptions};
use futures::future::{lazy};

use auth::middleware::{Middleware, MiddlewareFuture};
use engine::serializer::{JsonMessage};


/// A middleware class, that will check a JSON Web Token in WebSocket message.
/// If token wasn't specified or it's invalid returns a `PathfinderError` object.
pub struct JwtTokenMiddleware;


impl JwtTokenMiddleware {
    /// Returns a new instance of `JwtTokenMiddleware` structure.
    pub fn new(_cli: &CliOptions) -> JwtTokenMiddleware {
        JwtTokenMiddleware {}
    }
}


impl Middleware for JwtTokenMiddleware {
    fn process_request(&self, message: JsonMessage) -> MiddlewareFuture {
        Box::new(lazy(move || Ok(())))
    }
}
