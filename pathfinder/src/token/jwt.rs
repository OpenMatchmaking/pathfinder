use super::super::error::{Result, PathfinderError};

use jsonwebtoken::{decode, TokenData, Validation};


#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub exp: i64
}


pub fn validate(token: &str, secret_key: &str) -> Result<TokenData<Claims>> {
    match decode::<Claims>(token, secret_key.as_bytes(), &Validation::default()) {
        Ok(valid_token) => Ok(valid_token),
        Err(err) => Err(PathfinderError::from(err))
    }
}