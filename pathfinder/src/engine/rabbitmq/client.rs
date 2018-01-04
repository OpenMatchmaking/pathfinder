//! An asynchronous RabbitMQ client for proxy engine
//!

use super::super::super::cli::{CliOptions};

/// A future-based asynchronous RabbitMQ client.
pub struct RabbitMQClient
{
    username: String,
    password: String,
    host: String,
    port: u16
}


impl RabbitMQClient {
    /// Returns a new instance of `RabbitMQClient`.
    pub fn new(cli: &CliOptions) -> RabbitMQClient {
        RabbitMQClient {
            username: cli.rabbitmq_username.clone(),
            password: cli.rabbitmq_password.clone(),
            host: cli.rabbitmq_ip.clone(),
            port: cli.rabbitmq_port
        }
    }

    /// Generates a connection URL to RabbitMQ broker.
    fn get_url_to_rabbitmq(&self) -> String {
        format!(
            "amqps://{}:{}@{}:{}/?heartbeat=10",
            self.username,
            self.password,
            self.host,
            self.port
        )
    }
}
