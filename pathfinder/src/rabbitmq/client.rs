//! An asynchronous RabbitMQ client for proxy engine
//!

use std::io;
use std::marker::{PhantomData};

use amq_protocol::uri::{AMQPUri};
use futures::future::{Future};
use lapin_futures_rustls::{AMQPConnectionRustlsExt};
use lapin_futures_rustls::lapin::channel::{Channel, ConfirmSelectOptions};
use lapin_futures_rustls::lapin::client::{Client, ConnectionOptions};
use lapin_futures_tls_internal::{AMQPStream as LapinAMQPStream};
use tokio_current_thread::{spawn};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_tcp::{TcpStream};
use tokio_tls_api::{TlsStream};

use super::super::cli::{CliOptions};
use super::super::error::{PathfinderError};


/// Alias for the lapin AMQP stream.
pub type AMQPStream = LapinAMQPStream<TlsStream<TcpStream>>;
/// Alias for the lapin client with TLS.
pub type LapinClient = Client<AMQPStream>;
/// Alias for the lapin future type.
pub type LapinFuture = Box<Future<Item=LapinClient, Error=io::Error> + 'static>;
/// Alias for generic future for pathfinder and RabbitMQ.
pub type RabbitMQFuture = Box<Future<Item=(), Error=PathfinderError> + 'static>;


/// A future-based asynchronous RabbitMQ client.
pub struct RabbitMQClient<'a, T: 'a>
{
    uri: AMQPUri,
    client: Option<LapinClient>,
    phantom: PhantomData<&'a T>
}


impl<'a, T: AsyncRead + AsyncWrite + Send + Sync + 'static> RabbitMQClient<'a, T> {
    /// Returns a new instance of `RabbitMQClient`.
    pub fn new(cli: &CliOptions) -> RabbitMQClient<'a, T> {
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
            client: None,
            phantom: PhantomData
        }
    }

    pub fn init(&mut self) {
        self.client = Some(self.create_client());
    }

    pub fn get_channel(&self) -> Box<Future<Item=Channel<AMQPStream>, Error=io::Error> + Send + 'static> {
        Box::new(
            self.client.unwrap().create_confirm_channel(ConfirmSelectOptions::default())
        )
    }

    fn get_client(&mut self) -> Option<LapinClient> {
        match self.client {
            Some(client) => Some(client),
            None => {
                self.client = Some(self.create_client());
                self.client
            }
        }
    }

    fn create_client(&self) -> LapinClient {
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
