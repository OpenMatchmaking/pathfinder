//! An asynchronous RabbitMQ client for proxy engine
//!

use std::sync::Arc;

use amq_protocol::uri::AMQPUri;
use failure::{err_msg, Error};
use futures::future::Future;
use futures::IntoFuture;
use lapin_futures::error::{Error as LapinError};
use lapin_futures_rustls::lapin::channel::{Channel, ConfirmSelectOptions};
use lapin_futures_rustls::lapin::client::{Client, ConnectionOptions};
use log::error;
use tokio::executor::spawn;
use tokio::net::TcpStream;

use crate::rabbitmq::utils::get_address_to_rabbitmq;

/// Alias for the lapin client with TLS.
pub type LapinClient = Client<TcpStream>;
/// Alias for the lapin channel.
pub type LapinChannel = Channel<TcpStream>;

/// Custom client context, stores data, channels and everything else
/// that can be used for communicating with AMQP.
pub struct RabbitMQContext {
    publish_channel: LapinChannel,
    consume_channel: LapinChannel
}

impl RabbitMQContext {
    pub fn new(publish_channel: LapinChannel, consume_channel: LapinChannel) -> RabbitMQContext {
        RabbitMQContext {
            publish_channel,
            consume_channel
        }
    }

    pub fn get_publish_channel(&self) -> LapinChannel {
        self.publish_channel.clone()
    }

    pub fn get_consume_channel(&self) -> LapinChannel {
        self.consume_channel.clone()
    }
}

/// A future-based asynchronous RabbitMQ client.
pub struct RabbitMQClient {
    client: Arc<LapinClient>
}

impl RabbitMQClient {
    /// Initializes the inner fields of RabbitMQ client for future usage.
    pub fn connect(uri: &AMQPUri) -> impl Future<Item=Self, Error=Error> + Sync + Send + 'static {
        let address = get_address_to_rabbitmq(uri);
        let uri_inner = uri.clone();

        TcpStream::connect(&address)
            .map_err(Error::from)
            .and_then(|stream| {
                Client::connect(stream, ConnectionOptions::from_uri(uri_inner))
                    .map_err(Error::from)
            })
            .and_then(|(client, heartbeat)| {
                spawn(heartbeat.map_err(|err| error!("Heartbeat error: {}", err)))
                    .into_future()
                    .map(|_| RabbitMQClient { client: Arc::new(client) })
                    .map_err(|_| err_msg("Couldn't spawn the heartbeat task."))
            })
    }

    /// Returns client context as future, based on the lapin client instance.
    pub fn get_context(&self) -> impl Future<Item=RabbitMQContext, Error=LapinError> + Sync + Send + 'static {
        let client = self.client.clone();

        // Request channel for publishing messages
        client.create_confirm_channel(ConfirmSelectOptions::default())
            .map(|publish_channel| (client, publish_channel))
            .map(|(client, publish_channel)|
                // Request channel for consuming messages
                client.create_confirm_channel(ConfirmSelectOptions::default())
                    .map(|consume_channel| (publish_channel, consume_channel))
            )
            .flatten()
            // Initialize the client context
            .map(|(publish_channel, consume_channel)| 
                RabbitMQContext::new(publish_channel, consume_channel)
            )
    }
}
