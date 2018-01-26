//! An asynchronous RabbitMQ client for proxy engine
//!

use std::io;

use super::super::super::cli::{CliOptions};
use super::super::super::error::{PathfinderError};

use futures::future::{Future};
use lapin_futures_rustls::{AMQPConnectionRustlsExt, lapin};
use lapin_futures_tls_api::{AMQPStream};
use tokio_core::reactor::{Handle};


/// Alias for the lapin future type
pub type LapinFuture = Box<Future<Item=lapin::client::Client<AMQPStream>, Error=io::Error> + 'static>;

/// Alias for generic future for pathfinder and RabbitMQ
pub type RabbitMQFuture = Box<Future<Item=(), Error=PathfinderError> + 'static>;


/// A future-based asynchronous RabbitMQ client.
pub struct RabbitMQClient
{
    schema: String,
    username: String,
    password: String,
    host: String,
    port: u16,
    virtual_host: String
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
            host: cli.rabbitmq_host.clone(),
            port: cli.rabbitmq_port,
            virtual_host: cli.rabbitmq_virtual_host.clone()
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
            "{}://{}:{}@{}:{}/{}?heartbeat=10",
            self.schema,
            self.username,
            self.password,
            self.host,
            self.port,
            self.virtual_host
        )
    }
}
