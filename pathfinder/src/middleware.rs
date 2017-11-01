use super::error::{Result};


pub trait Middleware {
    /// Applied transforms and checks to an incoming request. If it failed,
    /// then should return an PathfinderError instance.
    fn process_request(&self) -> Result<()>;
}


/// Default class which is used for reverse proxy without authentication validator.
pub struct EmptyMiddleware {}


impl EmptyMiddleware {
    pub fn new() -> EmptyMiddleware {
        EmptyMiddleware {}
    }
}


impl Middleware for EmptyMiddleware {
    fn process_request(&self) -> Result<()> {
        println!("Empty!");
        Ok(())
    }
}


/// A middleware class, that will check a specified token in WebSocket
/// headers. Otherwise returns an error, if it isn't specified or invalid.
pub struct AuthTokenMiddleware {}


impl AuthTokenMiddleware {
    pub fn new() -> AuthTokenMiddleware {
        AuthTokenMiddleware {}
    }
}


impl Middleware for AuthTokenMiddleware {
    fn process_request(&self) -> Result<()> {
        println!("Secured!");
        Ok(())
    }
}
