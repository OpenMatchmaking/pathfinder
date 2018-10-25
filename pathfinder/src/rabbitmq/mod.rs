//! An asynchronous RabbitMQ client
//!
pub mod client;

pub use self::client::{
    LapinClient,
    LapinChannel,
    RabbitMQFuture,
    RabbitMQClient,
};
