//! This modules contains constants and type aliases for midddlewares.
//!

pub mod base;
pub mod empty;
pub mod jwt;
pub mod utils;

// For more details about used exchanges and routing keys look in the
// Open Matchmaking documentation on GitHub.
pub const TOKEN_VERIFY_ROUTING_KEY: &'static str = "auth.token.verify";
pub const TOKEN_VERIFY_EXCHANGE: &'static str = "open-matchmaking.auth.token.verify.direct";
pub const TOKEN_USER_PROFILE_ROUTING_KEY: &'static str = "auth.users.retrieve";
pub const TOKEN_USER_PROFILE_EXCHANGE: &'static str = "open-matchmaking.auth.users.retrieve.direct";

pub use self::base::{Middleware, MiddlewareFuture, CustomUserHeaders};
pub use self::empty::EmptyMiddleware;
pub use self::jwt::JwtTokenMiddleware;
pub use self::utils::get_permissions;
