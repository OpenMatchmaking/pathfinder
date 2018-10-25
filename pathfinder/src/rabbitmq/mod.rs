//! An asynchronous RabbitMQ client
//!
pub mod client;

pub use self::client::{
    AMQPStream,
    LapinClient,
    LapinFuture,
    RabbitMQFuture,
    RabbitMQClient,
};
