use std::vec::{Vec};

use super::error::{Result};

use cli::{CliOptions};
use tokio_core::reactor::{Handle};
use tungstenite::handshake::server::{Request};


pub type WebSocketHeaders = Option<Vec<(String, String)>>;


pub trait Middleware {
    /// Applied transforms and checks to an incoming request. If it failed,
    /// then should return an PathfinderError instance.
    fn process_request(&self, request: &Request, handle: &Handle) -> Result<WebSocketHeaders>;
}


/// Default class which is used for reverse proxy without an authentication
/// header validation process.
pub struct EmptyMiddleware;


impl EmptyMiddleware {
    pub fn new(_cli: &CliOptions) -> EmptyMiddleware {
        EmptyMiddleware {}
    }
}


impl Middleware for EmptyMiddleware {
    fn process_request(&self, _request: &Request, _handle: &Handle) -> Result<WebSocketHeaders> {
        Ok(None)
    }
}
