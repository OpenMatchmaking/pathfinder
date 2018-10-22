//! An asynchronous RabbitMQ client for proxy engine
//!

use std::io;

use super::super::super::cli::{CliOptions};
use super::super::super::error::{PathfinderError};

use amq_protocol::uri::{AMQPUri};
use futures::future::{Future};
use lapin_futures_rustls::{AMQPConnectionRustlsExt};
use lapin_futures_rustls::lapin::client::{Client, ConnectionOptions};
use lapin_futures_tls_internal::{AMQPStream};
use tokio::reactor::{Handle};
use tokio_current_thread::{spawn};
use tokio_tcp::{TcpStream};
use tokio_tls_api::{TlsStream};


/// Alias for the Lapin client with TLS.
pub type LapinClient = Client<AMQPStream<TlsStream<TcpStream>>>;
/// Alias for the lapin future type.
pub type LapinFuture = Box<Future<Item=LapinClient, Error=io::Error> + 'static>;
/// Alias for generic future for pathfinder and RabbitMQ.
pub type RabbitMQFuture = Box<Future<Item=(), Error=PathfinderError> + 'static>;


/// A future-based asynchronous RabbitMQ client.
pub struct RabbitMQClient
{
    uri: AMQPUri
}


impl RabbitMQClient {
    /// Returns a new instance of `RabbitMQClient`.
    pub fn new(cli: &CliOptions) -> RabbitMQClient {
        let schema = match cli.rabbitmq_secured {
            true => "amqps",
            false => "amqp",
        };

        let uri = format!(
            "{}://{}:{}@{}:{}/{}",
            schema.to_string(),
            cli.rabbitmq_username.clone(),
            cli.rabbitmq_password.clone(),
            cli.rabbitmq_host.clone(),
            cli.rabbitmq_port,
            cli.rabbitmq_virtual_host.clone()
        );

        RabbitMQClient {
            uri: uri.parse().unwrap()
        }
    }

    /// Returns a `lapin_futures::client::Client` instance wrapped in a `Future`.
    pub fn get_future(&self, handle: &Handle) -> LapinFuture {
        let address = self.get_address_to_rabbitmq().parse().unwrap();
        
        TcpStream::connect(&address).and_then(|stream| {
            Client::connect(stream, ConnectionOptions::from_uri(self.uri))
        })
        .and_then(|(client, heartbeat)| {
            spawn(heartbeat.map_err(|err| eprintln!("Heartbeat error: {:?}", err)))
                .into_future()
                .map(|_| client)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Spawn error."))
        })
    }

    /// Generates a connection URL to RabbitMQ broker.
    fn get_address_to_rabbitmq(&self) -> String {
        format!("{}:{}", self.uri.authority.host, self.uri.authority.port)
    }
}
