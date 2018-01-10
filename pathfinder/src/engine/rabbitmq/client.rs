//! An asynchronous RabbitMQ client for proxy engine
//!

use std::io;

use super::super::super::cli::{CliOptions};

use futures::future::{Future};
use lapin_futures_rustls::{AMQPConnectionRustlsExt, lapin};
use lapin_futures_tls_api::{AMQPStream};
use tokio_core::reactor::{Handle};


/// Alias for the lapin future type
pub type LapinFuture = Box<Future<Item=lapin::client::Client<AMQPStream>, Error=io::Error> + 'static>;


/// A future-based asynchronous RabbitMQ client.
pub struct RabbitMQClient
{
    schema: String,
    username: String,
    password: String,
    host: String,
    port: u16
}


impl RabbitMQClient {
    /// Returns a new instance of `RabbitMQClient`.
    pub fn new(cli: &CliOptions) -> RabbitMQClient {
        let schema = match cli.rabbitmq_secured {
            true => "amqps",
            false => "amqp",
        };

        RabbitMQClient {
            schema: schema.to_string(),
            username: cli.rabbitmq_username.clone(),
            password: cli.rabbitmq_password.clone(),
            host: cli.rabbitmq_ip.clone(),
            port: cli.rabbitmq_port
        }
    }

    /// Returns a `lapin_futures::client::Client` instance wrapped in a `Future`
    pub fn get_future(&self, handle: &Handle) -> LapinFuture {
        let uri = self.get_url_to_rabbitmq();
        uri.connect(handle.clone(), |_| ())
    }

    /// Generates a connection URL to RabbitMQ broker.
    fn get_url_to_rabbitmq(&self) -> String {
        format!(
            "{}://{}:{}@{}:{}/?heartbeat=10",
            self.schema,
            self.username,
            self.password,
            self.host,
            self.port
        )
    }
}
