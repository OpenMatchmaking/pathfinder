pub mod engine;
pub mod futures;
pub mod middleware;
pub mod router;
pub mod options;
pub mod serializer;
pub mod utils;

use std::sync::Arc;

use futures::sync::mpsc;
use tungstenite::Message;

/// Default AMQP exchange point for requests
pub const REQUEST_EXCHANGE: &'static str = "open-matchmaking.direct";
/// Default AMQP exchange point for responses
pub const RESPONSE_EXCHANGE: &'static str = "open-matchmaking.responses.direct";

/// Alias type for msps sender.
pub type MessageSender = Arc<mpsc::UnboundedSender<Message>>;

pub use self::engine::{Engine};
pub use self::futures::rpc_request_future;
pub use self::middleware::{
    EmptyMiddleware,
    JwtTokenMiddleware,
    Middleware,
    MiddlewareFuture
};
pub use self::router::{extract_endpoints, Endpoint, ReadOnlyEndpoint, Router};
pub use self::options::{RpcOptions};
pub use self::serializer::{JsonMessage, Serializer};
pub use self::utils::{deserialize_message, serialize_message, wrap_a_string_error};
