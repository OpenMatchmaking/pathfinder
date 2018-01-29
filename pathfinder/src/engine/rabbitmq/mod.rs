//! An asynchronous RabbitMQ client
//!
pub mod client;

pub use self::client::{RabbitMQClient, LapinFuture, RabbitMQFuture};
