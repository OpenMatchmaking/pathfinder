//! An asynchronous RabbitMQ client for proxy engine
//!

use std::io;

use amq_protocol::uri::{AMQPUri};
use futures::{IntoFuture};
use futures::future::{Future};
use lapin_futures_rustls::{AMQPConnectionRustlsExt};
use lapin_futures_rustls::lapin::channel::{Channel, ConfirmSelectOptions};
use lapin_futures_rustls::lapin::client::{Client, ConnectionOptions};
use tokio::executor::{spawn};
use tokio_tcp::{TcpStream};

use super::super::cli::{CliOptions};
use super::super::error::{PathfinderError};


/// Alias for the lapin client with TLS.
pub type LapinClient = Client<TcpStream>;
/// Alias for the lapin channel.
pub type LapinChannel = Channel<TcpStream>;
/// Alias for generic future for pathfinder and RabbitMQ.
pub type RabbitMQFuture = Box<Future<Item=(), Error=PathfinderError> + 'static>;


/// A future-based asynchronous RabbitMQ client.
pub struct RabbitMQClient
{
    uri: AMQPUri,
    client: Option<Box<Future<Item=Client<TcpStream>, Error=io::Error> + Sync + Send + 'static>>,
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
            uri: uri.parse().unwrap(),
            client: None
        }
    }

    pub fn init(&mut self) {
        self.client = self.get_client();
    }

    pub fn get_channel(&self) -> Box<Future<Item=LapinChannel, Error=io::Error> + Sync + Send + 'static> {
        Box::new(
            self.client.unwrap().and_then(|client| {
                client.create_confirm_channel(ConfirmSelectOptions::default())
            })
        )
    }

    fn get_client(&mut self) -> Option<Box<Future<Item=LapinClient, Error=io::Error> + Sync + Send + 'static>> {
        match self.client {
            Some(client) => Some(client),
            None => {
                self.client = Some(Box::new(self.create_client()));
                self.client
            }
        }
    }

    fn create_client(&self) -> impl Future<Item=LapinClient, Error=io::Error> + Sync + Send + 'static {
        let address = self.get_address_to_rabbitmq().parse().unwrap();
        let uri = self.uri.clone();

        TcpStream::connect(&address).and_then(|stream| {
            Client::connect(stream, ConnectionOptions::from_uri(uri))
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
