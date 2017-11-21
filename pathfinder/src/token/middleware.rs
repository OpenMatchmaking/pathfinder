use std::str;
use std::vec::{Vec};

use super::super::error::{Result, PathfinderError};
use super::super::middleware::{Middleware};

use cli::{CliOptions};
use tungstenite::handshake::server::{Request};


/// A middleware class, that will check a specified token in WebSocket
/// headers. Otherwise returns an error, if it isn't specified or invalid.
pub struct JwtTokenMiddleware;


impl JwtTokenMiddleware {
    pub fn new(_cli: &CliOptions) -> JwtTokenMiddleware {
        JwtTokenMiddleware { }
    }
}


impl Middleware for JwtTokenMiddleware {
    fn process_request(&self, request: &Request) -> Result<Option<Vec<(String, String)>>> {
        match request.headers.find_first("Sec-WebSocket-Protocol") {
             Some(token) => {
                 let parsed_value = str::from_utf8(token).unwrap();
                 let extra_headers = vec![
                    (String::from("Sec-WebSocket-Protocol"), String::from(parsed_value))
                 ];
                 Ok(Some(extra_headers))
             },
             None => {
                 let message = String::from("Token was not found");
                 Err(PathfinderError::AuthenticationError(message))
             }
        }
    }
}
