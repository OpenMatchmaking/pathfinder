//! Authentication / authorization layer
//!

pub mod base;
pub mod empty;
pub mod jwt;

// For more details about resources look into the Open Matchmaking documentation on GitHub.
pub const TOKEN_VERIFY_ROUTING_KEY: &'static str = "auth.token.verify";
pub const TOKEN_VERIFY_EXCHANGE: &'static str = "open-matchmaking.auth.token.verify.direct";

pub use self::base::{Middleware, MiddlewareFuture};
pub use self::empty::EmptyMiddleware;
pub use self::jwt::JwtTokenMiddleware;
