use std::str;

use super::error::{Result, PathfinderError};

use cli::{CliOptions};
use tungstenite::handshake::server::{Request};


pub trait Middleware {
    /// Applied transforms and checks to an incoming request. If it failed,
    /// then should return an PathfinderError instance.
    fn process_request(&self, request: &Request) -> Result<()>;
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
    fn process_request(&self, _request: &Request) -> Result<()> {
        Ok(())
    }
}


/// A middleware class, that will check a specified token in WebSocket
/// headers. Otherwise returns an error, if it isn't specified or invalid.
pub struct AuthTokenMiddleware;


impl AuthTokenMiddleware {
    pub fn new(_cli: &CliOptions) -> AuthTokenMiddleware {
        AuthTokenMiddleware { }
    }
}


impl Middleware for AuthTokenMiddleware {
    fn process_request(&self, request: &Request) -> Result<()> {
        match request.headers.find_first("Sec-WebSocket-Protocol") {
             Some(token) => {
                 let parsed_value = str::from_utf8(token).unwrap();
                 println!("Sec-WebSocket-Protocol: {}", parsed_value);
                 Ok(())
             },
             None => {
                 let message = String::from("Token was not found");
                 Err(PathfinderError::AuthenticationError(message))
             }
        }
    }
}
