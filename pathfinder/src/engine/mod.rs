//! Proxy engine
//!
//! This module is intended for processing incoming requests from clients,
//! handling occurred errors during a work, communicating with a message
//! broker and preparing appropriate responses in the certain format.
//!

#[macro_use]
pub mod engine_macro;
pub mod rabbitmq;
pub mod router;
pub mod serializer;

use std::cell::{RefCell};
use std::collections::{HashMap};
use std::net::{SocketAddr};
use std::rc::{Rc};
use std::str::{from_utf8};
use std::vec::Vec;

pub use self::router::{Router, Endpoint, extract_endpoints};
pub use self::serializer::{Serializer};
pub use self::rabbitmq::{RabbitMQClient, RabbitMQFuture};

use super::cli::{CliOptions};
use super::error::{Result, PathfinderError};

use json::{parse as json_parse};
use futures::{Stream};
use futures::future::{Future};
use futures::sync::{mpsc};
use lapin_futures_rustls::lapin::types::{AMQPValue, FieldTable};
use lapin_futures_rustls::lapin::channel::{ConfirmSelectOptions};
use lapin_futures_rustls::lapin::channel::{BasicPublishOptions, BasicProperties, BasicConsumeOptions};
use lapin_futures_rustls::lapin::channel::{QueueDeclareOptions, QueueDeleteOptions, QueueBindOptions};
use json::{JsonValue};
use tokio_core::reactor::{Handle};
use tungstenite::{Message};
use uuid::{Uuid};


/// Type alias for dictionary with `SocketAddr` as a key and `UnboundedSender<Message>` as a value.
pub type ActiveConnections = Rc<RefCell<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>;

/// Default AMQP exchange point for requests
const REQUEST_EXCHANGE: &'static str = "open-matchmaking.direct";
/// Default AMQP exchange point for responses
const RESPONSE_EXCHANGE: &'static str = "open-matchmaking.responses.direct";


/// Proxy engine for processing messages, handling errors and communicating with a message broker.
pub struct Engine {
    router: Rc<Box<Router>>,
    rabbitmq_client: Rc<RefCell<Box<RabbitMQClient>>>
}


impl Engine {
    /// Returns a new instance of `Engine`.
    pub fn new(cli: &CliOptions, router: Box<Router>) -> Engine {
        Engine {
            router: Rc::new(router),
            rabbitmq_client: Rc::new(RefCell::new(Box::new(RabbitMQClient::new(cli))))
        }
    }

    /// Main handler for generating a response per each incoming request.
    pub fn handle(&self, message: Box<JsonValue>, client: &SocketAddr, connections: &ActiveConnections, handle: &Handle) -> RabbitMQFuture {
        let url = message["url"].as_str().unwrap();
        let queue_name = format!("{}", Uuid::new_v4());
        let event_name = message["event-name"].as_str().unwrap_or("null");
        let endpoint = self.router.clone().match_url_or_default(url);

        let request_headers = self.prepare_request_headers(message, endpoint);
        let client_future = self.rabbitmq_client.borrow().get_future(handle);

        Box::new(client_future
            // 1. Create a channel
            .and_then(|client| {
                client.create_confirm_channel(ConfirmSelectOptions::default())
            })
            .map_err(|err| {
                let message = format!("Error during creating a channel: {}", err);
                error!("{}", message);
                err
            })

            // 2. Declare a response queue
            .and_then(|channel| {
                let queue_declare_options = QueueDeclareOptions {
                    passive: false,
                    durable: true,
                    exclusive: true,
                    auto_delete: false,
                    ..Default::default()
                };

                channel.queue_declare(&queue_name, &queue_declare_options, &FieldTable::new())
                    .map(|_| channel)
            })
            .map_err(|err| {
                let message = format!("Error during declaring the queue: {}", err);
                error!("{}", message);
                err
            })

            // 3. Link the response queue the exchange
            .and_then(|channel| {
                channel.queue_bind(
                    &queue_name,
                    &RESPONSE_EXCHANGE,
                    &endpoint.get_microservice(),
                    &QueueBindOptions::default(),
                    &FieldTable::new()
                )
                    .map(|_| channel)
            })
            .map_err(|err| {
                let message = format!("Error during linking the response queue with exchange: {}", err);
                error!("{}", message);
                err
            })

            // 4. Publish message into the microservice queue and make ensure that it's delivered
            // TODO: Make ensure that for testing endpoint and exchange are defined via RabbitMQ Management plugin
            .and_then(|channel| {
                let publish_message_options = BasicPublishOptions {
                    mandatory: true,
                    immediate: false,
                    ..Default::default()
                };

                let mut prepared_headers = FieldTable::new();
                for &(key, value) in request_headers.iter() {
                    prepared_headers.insert(key, AMQPValue::LongString(value));
                }

                let basic_properties = BasicProperties {
                    content_type: Some("application/json".to_string()),
                    headers: Some(prepared_headers),              // Headers per each message
                    delivery_mode: Some(2),                       // Message must be persistent
                    reply_to: Some(queue_name.to_string()),       // Response queue
                    correlation_id: Some(event_name.to_string()), // Event name
                    ..Default::default()
                };

                channel.basic_publish(
                    &REQUEST_EXCHANGE,
                    &endpoint.get_microservice(),
                    message["content"].dump().as_bytes(),
                    &publish_message_options,
                    basic_properties
                )
                    .map(|_confirmation| channel)
            })
            .map_err(|err| {
                let message = format!("Error during publishing a message: {}", err);
                error!("{}", message);
                err
            })

            // 5. Consume a response message from the queue, that was declared on the 2nd step
            .and_then(|channel| {
                channel.basic_consume(
                    &queue_name,
                    "response_consumer",
                    &BasicConsumeOptions::default(),
                    &FieldTable::new()
                )
                    .and_then(move |stream| {
                        stream.take(1)
                              .into_future()
                              .map_err(|(err, _)| err)
                              .map(move |(message, _)| (channel, message.unwrap()))
                    })
            })
            .map_err(|err| {
                let message = format!("Error during consuming the response message: {}", err);
                error!("{}", message);
                err
            })

            // 6. Prepare a response for a client, serialize and sent via WebSocket transmitter
            .and_then(|(channel, message)| {
                let raw_data = from_utf8(&message.data).unwrap();
                let json = Box::new(json_parse(raw_data).unwrap());
                let response = self.prepare_response(json);
                let transmitter = &connections.borrow_mut()[&client];
                transmitter.unbounded_send(response).unwrap();
                channel.basic_ack(message.delivery_tag)
                    .map(move |_| channel)
            })
            .map_err(|err| {
                let message = format!("Error during sending a message to the client: {}", err);
                error!("{}", message);
                err
            })

            // 7. Unbind the response queue from the exchange point
            // TODO: Uncomment this code when my PR for queue_unbind will be merged
            //.and_then(|channel| {
            //    channel.queue_unbind(
            //        &queue_name,
            //        &response_exchange_name,
            //        &endpoint.get_microservice(),
            //        &QueueUnbindOptions::default(),
            //        &FieldTable::new()
            //    )
            //        .map(|_| channel)
            //})
            //.map_err(|err| {
            //    let message = format!("Error during linking the response queue with exchange: {}", err);
            //    error!("{}", message);
            //    Err(PathfinderError::Io(err))
            //})

            // 8. Delete the response queue
            .and_then(|channel| {
                let queue_delete_options = QueueDeleteOptions {
                    if_unused: false,
                    if_empty: false,
                    ..Default::default()
                };

                channel.queue_delete(&queue_name, &queue_delete_options)
                    .map(|_| channel)
            })
            .map_err(|err| {
                let message = format!("Error during deleting the queue: {}", err);
                error!("{}", message);
                err
            })

            // 9. Close the channel
            .and_then(|channel| {
                channel.close(200, "Close the channel.")
            })
            .map_err(|err| {
                let message = format!("Error during closing the channel: {}", err);
                error!("{}", message);
                err
            })

            .then(|result| {
                match result {
                    Ok(_) => Ok(()),
                    Err(err) => Err(PathfinderError::Io(err))
                }
            })
        )
    }

    /// Transforms an error (which is a string) into JSON object in the special format.
    pub fn wrap_an_error(&self, err: &str) -> Message {
        let json_error_message = object!("details" => err);
        let serializer = Serializer::new();
        serializer.serialize(json_error_message.dump()).unwrap()
    }

    /// Serialize a JSON object into message.
    pub fn serialize_message(&self, json: Box<JsonValue>) -> Message {
        let serializer = Serializer::new();
        serializer.serialize(json.dump()).unwrap()
    }

    /// Deserialize a message into JSON object.
    pub fn deserialize_message(&self, message: &Message) -> Result<Box<JsonValue>> {
        let serializer = Serializer::new();
        serializer.deserialize(message)
    }

    /// Prepares a list of key-value pairs for headers in message.
    fn prepare_request_headers(&self, json: Box<JsonValue>, endpoint: Box<Endpoint>) -> Box<Vec<(String, String)>> {
        Box::new(vec![
            (String::from("Microservice-Name"), endpoint.get_microservice()),
            (String::from("Request-URI"), endpoint.get_url()),
            (String::from("Event-Name"), json["event-name"].as_str().unwrap_or("null").to_string()),
            (String::from("Token"), json["token"].as_str().unwrap_or("null").to_string())
        ])
    }

    /// Converts a JSON object into the `tungstenite::Message` message.
    fn prepare_response(&self, json: Box<JsonValue>) -> Message {
        self.serialize_message(json)
    }
}
