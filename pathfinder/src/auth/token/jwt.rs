//! Wrappers for validating a JSON Web Token
//!

use super::super::super::error::{Result, PathfinderError};

use jsonwebtoken::{decode, TokenData, Validation};

/// A default issuer in JSON Web Token
pub const DEFAULT_ISSUER: &'static str = "pathfinder";


/// A struct with validated fields in JSON Web Token
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub exp: i64,
}


/// Validates a passed token with the certain secret key and a validation structure.
pub fn validate(token: &str, secret_key: &str, validation_struct: &Validation) -> Result<TokenData<Claims>> {
    match decode::<Claims>(token, secret_key.as_bytes(), &validation_struct) {
        Ok(valid_token) => Ok(valid_token),
        Err(err) => Err(PathfinderError::from(err))
    }
}
