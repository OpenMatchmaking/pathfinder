//! An asynchronous RabbitMQ client
//!

pub mod client;
pub mod utils;

pub use self::client::{LapinChannel, LapinClient, RabbitMQClient};
pub use self::utils::{get_address_to_rabbitmq, get_uri};
