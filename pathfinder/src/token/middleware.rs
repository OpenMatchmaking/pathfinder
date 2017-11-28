use std::str;
use std::vec::{Vec};

use super::jwt::{DEFAULT_ISSUER, validate as validate_token};
use super::super::error::{Result, PathfinderError};
use super::super::middleware::{Middleware};

use cli::{CliOptions};
use jsonwebtoken::{Validation, Algorithm};
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

    fn get_user_id(&self) -> Result<String> {
        Ok(String::from("test"))
//        Err(_) => {
//            let message = String::from("Token is expired or doesn't exist.");
//            Err(PathfinderError::AuthenticationError(message))
//        }
    }
}


impl Middleware for JwtTokenMiddleware {
    fn process_request(&self, request: &Request) -> Result<Option<Vec<(String, String)>>> {
        match request.headers.find_first("Sec-WebSocket-Protocol") {
             Some(raw_token) => {
                 // Try to fetch token after handshake
                 let extracted_token = self.extract_token_from_header(raw_token)?;

                 // Validate the passed token with request
                 let user_id = self.get_user_id()?;
                 let validation_struct = self.get_validation_struct(user_id);
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
