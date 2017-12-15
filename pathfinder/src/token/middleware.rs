use std::str;

use super::jwt::{DEFAULT_ISSUER, validate};
use super::super::error::{Result, PathfinderError};
use super::super::middleware::{JsonMessage, Middleware, MiddlewareFuture};

use cli::{CliOptions};
use futures::{Future};
use futures::future::lazy;
use jsonwebtoken::{Validation, Algorithm};
use tokio_core::reactor::{Handle};
use redis_async::client::{paired_connect};


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

//    fn get_user_id(&self, raw_token: &str, handle: &Handle) -> Box<Future<Item=PairedConnection, Error=RedisError> + 'static> {
//        let redis_socket_address = self.redis_address.parse().unwrap();
//        let redis_connection = paired_connect(&redis_socket_address, handle);
//
//        // TODO: Add AUTH query if a password was specified.
//        redis_connection
//            // Get the User ID from Redis by the token
//            .and_then(move |connection| {
//                connection.send::<String>(resp_array!["GET", String::from(raw_token)])
//            })
//    }
}


impl Middleware for JwtTokenMiddleware {
    fn process_request(&self, message: &JsonMessage, handle: &Handle) -> MiddlewareFuture {
        let token = match message["token"].as_str() {
            Some(token) => String::from(token),
            None => {
                return Box::new(lazy(move || {
                    let message = String::from("Token was not specified.");
                    Err(PathfinderError::AuthenticationError(message))
                }))
            }
        };

        let redis_socket_address = self.redis_address.parse().unwrap();
        let redis_connection = paired_connect(&redis_socket_address, handle);
        Box::new(
            redis_connection
                // Get the User ID from Redis by the token
                .and_then(move |connection| {
                    connection.send::<String>(resp_array!["GET", token])
                })
                // Connection issue or token is already deleted
                .map_err(|err| {
                    let message = String::from("Token is expired.");
                    PathfinderError::AuthenticationError(message)
                })
                // Extracted user_id used here for additional JWT validation
                .map(|user_id| {
                    let validation_struct = self.get_validation_struct(&user_id);
                    validate(&token, &self.jwt_secret, &validation_struct)
                })
                .map_err(|_| {
                    let message = String::from("Token is invalid.");
                    PathfinderError::AuthenticationError(message)
                })
                .map(|_| ())
        )
    }
}
