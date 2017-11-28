use std::str;
use std::vec::{Vec};

use super::super::error::{Result, PathfinderError};
use super::super::middleware::{Middleware};

use cli::{CliOptions};
use tungstenite::handshake::server::{Request};


/// A middleware class, that will check a specified token in WebSocket
/// headers. Otherwise returns an error, if it isn't specified or invalid.
pub struct JwtTokenMiddleware {
    jwt_secret: String
}


impl JwtTokenMiddleware {
    pub fn new(cli: &CliOptions) -> JwtTokenMiddleware {
        JwtTokenMiddleware {
            jwt_secret: cli.jwt_secret_key.clone()
        }
    }

    fn extract_token_from_header(&self, raw_token: &[u8]) -> Result<String> {
        match str::from_utf8(raw_token) {
            Ok(parsed_value) => Ok(String::from(parsed_value)),
            Err(_) => {
                let message = String::from("Token is invalid.");
                Err(PathfinderError::AuthenticationError(message))
            }
        }
    }
}


impl Middleware for JwtTokenMiddleware {
    fn process_request(&self, request: &Request) -> Result<Option<Vec<(String, String)>>> {
        match request.headers.find_first("Sec-WebSocket-Protocol") {
             Some(raw_token) => {
                 let token = self.extract_token_from_header(raw_token)?;
                 let extra_headers = vec![(String::from("Sec-WebSocket-Protocol"), token)];
                 Ok(Some(extra_headers))
             },
             None => {
                 let message = String::from("Token was not found");
                 Err(PathfinderError::AuthenticationError(message))
             }
        }
    }
}
