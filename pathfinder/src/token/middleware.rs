use std::str;
use std::vec::{Vec};

use super::jwt::{DEFAULT_ISSUER, validate as validate_token};
use super::super::error::{Result, PathfinderError};
use super::super::middleware::{Middleware, WebSocketHeaders};

use cli::{CliOptions};
use futures::{Future};
use futures::sync::{oneshot};
use jsonwebtoken::{Validation, Algorithm};
use tokio_core::reactor::{Handle};
use tungstenite::handshake::server::{Request};
use redis_async::client::{paired_connect, PairedConnection};
use redis_async::error::{Error as RedisError};


type PairedConnectionBox = Box<Future<Item=PairedConnection, Error=RedisError>>;


/// A middleware class, that will check a specified token in WebSocket
/// headers. Otherwise returns an error, if it isn't specified or invalid.
pub struct JwtTokenMiddleware {
    jwt_secret: String,
    redis_address: String,
    redis_password: Option<String>,
}


impl JwtTokenMiddleware {
    pub fn new(cli: &CliOptions) -> JwtTokenMiddleware {
        let mut redis_password = None;
        if cli.redis_password != "" {
            redis_password = Some(cli.redis_password.clone())
        }

        JwtTokenMiddleware {
            jwt_secret: cli.jwt_secret_key.clone(),
            redis_address: format!("{}:{}", cli.redis_ip, cli.redis_port),
            redis_password: redis_password
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

    fn get_validation_struct(&self, user_id: &str) -> Validation {
        let mut validation = Validation {
            leeway: 0,
            validate_exp: true,
            validate_iat: true,
            validate_nbf: true,
            iss: Some(String::from(DEFAULT_ISSUER)),
            sub: None,
            aud: None,
            algorithms: Some(vec![Algorithm::HS512]),
        };
        validation.set_audience(&user_id);
        validation
    }

    fn get_user_id(&self, raw_token: String, handle: &Handle) -> Result<String> {
        let redis_socket_address = self.redis_address.parse().unwrap();
        let redis_connection = paired_connect(&redis_socket_address, handle);

        // Make the authentication before, if a password was specified.
        let get_user_id_future = redis_connection.and_then(move |connection| {
            // Check credentials
            // Get the User ID from Redis by the token
            connection.send::<String>(resp_array!["GET", raw_token])
        });

        let mut processing_result;
        handle.spawn(get_user_id_future.then(move |response| {
            processing_result = match response {
                Ok(user_id) => Ok(String::from(user_id)),
                Err(_) => {
                    let message = String::from("Token is expired or doesn't exist.");
                    Err(PathfinderError::AuthenticationError(message))
                }
            };
            Ok(())
        }));

        processing_result
    }
}


impl Middleware for JwtTokenMiddleware {
    fn process_request(&self, request: &Request, handle: &Handle) -> Result<WebSocketHeaders> {
        match request.headers.find_first("Sec-WebSocket-Protocol") {
             Some(raw_token) => {
                 // Try to fetch token after handshake
                 let extracted_token = self.extract_token_from_header(raw_token)?;

                 // Validate the passed token with request
                 let user_id = self.get_user_id(extracted_token.clone(), handle)?;
                 let validation_struct = self.get_validation_struct(&user_id);
                 let _token = validate_token(&extracted_token, &self.jwt_secret, &validation_struct)?;

                 // Return validated header as is
                 let extra_headers = vec![(String::from("Sec-WebSocket-Protocol"), extracted_token)];
                 Ok(Some(extra_headers))
             },
             None => {
                 let message = String::from("Token was not specified.");
                 Err(PathfinderError::AuthenticationError(message))
             }
        }
    }
}
