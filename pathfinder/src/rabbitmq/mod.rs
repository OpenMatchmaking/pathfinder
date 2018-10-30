//! An asynchronous RabbitMQ client
//!
pub mod client;
pub mod utils;

pub use self::client::{
    LapinClient,
    LapinChannel,
    RabbitMQFuture,
    RabbitMQClient,
};
pub use self::utils::{
    get_uri,
    get_address_to_rabbitmq,
};
