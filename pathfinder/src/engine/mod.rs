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

pub use self::router::{Router, Endpoint, extract_endpoints};
pub use self::serializer::{Serializer};
pub use self::rabbitmq::{RabbitMQClient, LapinFuture};

use super::cli::{CliOptions};
use super::error::{Result};

use futures::{Stream};
use futures::future::{Future};
use futures::sync::{mpsc};
use lapin_futures_rustls::lapin;
use lapin::types::FieldTable;
use lapin::channel::{ConfirmSelectOptions};
use lapin::channel::{BasicPublishOptions, BasicProperties, BasicConsumeOptions};
use lapin::channel::{QueueDeclareOptions, QueueDeleteOptions, QueueBindOptions};
use json::{JsonValue};
use tokio_core::reactor::{Handle};
use tungstenite::{Message};
use uuid::{Uuid};


/// Type alias for dictionary with `SocketAddr` as a key and `UnboundedSender<Message>` as a value.
pub type ActiveConnections = Rc<RefCell<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>;

/// Default AMQP exchange point for requests
pub const REQUEST_EXCHANGE: &'static str = "open-matchmaking.direct";
/// Default AMQP exchange point for responses
pub const RESPONSE_EXCHANGE: &'static str = "open-matchmaking.responses.direct";


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
    pub fn handle(&self, message: Box<JsonValue>, client: &SocketAddr, connections: &ActiveConnections, handle: &Handle) -> LapinFuture {
        let queue_name = format!("{}", Uuid::new_v4());
        let client_future = self.rabbitmq_client.clone().get_future(handle);

        client_future
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
                    &matchmaking_endpoint,
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

                // TODO: Take headers from JSON and set headers here?
                // TODO: Copy token from JSON into header
                let basic_properties = BasicProperties {
                    content_type: Some("application/json".to_string()),
                    delivery_mode: Some(2),                    // Message must be persistent
                    reply_to: Some(queue_name.to_string()),    // Response queue
                    correlation_id: Some("event".to_string()), // Event name
                    ..Default::default()
                };

                channel.basic_publish(
                    &REQUEST_EXCHANGE,
                    &matchmaking_endpoint,
                    b"test message",            // TODO: Extract JSON content and left it here
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
                              .map(move |(message, _)| (channel, message))
                    })

            })
            .map_err(|err| {
                let message = format!("Error during consuming the response message: {}", err);
                error!("{}", message);
                err
            })

            // 6. Prepare a response for a client, serialize and sent via WebSocket transmitter
            // TODO: Apply serialization for response and send it to client
            .and_then(|(channel, message)| {
                let message = message.unwrap();



                channel.basic_ack(message.delivery_tag);
                Ok(channel)
//                channel.basic_ack(message.delivery_tag)
//                    .map(move |_| channel)
            })
            .map_err(|err| {
                let message = format!("Error during sending a message to the client: {}", err);
                error!("{}", message);
                err
            })

            // 7. Unbind the response queue from the exchange point
            // TODO: Uncomment this code when my PR for queue_bind will be merged
            //.and_then(|channel| {
            //    channel.queue_unbind(
            //        &queue_name,
            //        &response_exchange_name,
            //        &matchmaking_endpoint,
            //        &QueueUnbindOptions::default(),
            //        &FieldTable::new()
            //    )
            //        .map(|_| channel)
            //})
            //.map_err(|err| {
            //    let message = format!("Error during linking the response queue with exchange: {}", err);
            //    error!("{}", message);
            //    err
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

        //let transmitter = &connections.borrow_mut()[&client];
        //let request = self.prepare_request(message);

        //println!("{}", request);
        //let response = self.prepare_response(request);
        //transmitter.unbounded_send(response).unwrap();
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

    /// Converts a JSON object into a message for a message broker.
    fn prepare_request(&self, json: Box<JsonValue>) -> Box<JsonValue> {
        let mut request = Box::new(object!{
            "headers" => object!{},
            "content" => object!{}
        });

        self.generate_request_headers(&mut request, json);
        request
    }

    /// Converts a JSON object into the `tungstenite::Message` message.
    fn prepare_response(&self, json: Box<JsonValue>) -> Message {
        self.serialize_message(json)
    }

    /// Transforms a request to the appropriate format for a further processing by a microservice.
    fn generate_request_headers(&self, request: &mut Box<JsonValue>, from_json: Box<JsonValue>) {
        request["headers"]["request-id"] = format!("{}", Uuid::new_v4()).into();

        let url = from_json["url"].as_str().unwrap();
        match self.router.match_url(url) {
            Ok(endpoint) => {
                request["headers"]["microservice-name"] = endpoint.get_microservice().into();
                request["headers"]["request-url"] = endpoint.get_url().into();
            },
            Err(_) => {
                request["headers"]["microservice-name"] = self.convert_url_into_microservice(url).into();
                request["headers"]["request-url"] = url.into();
            }
        }
    }

    /// Converts a URL to the certain microservice name, so that it will be used as a queue/topic name futher.
    fn convert_url_into_microservice(&self, url: &str) -> String {
        let mut external_url = url.clone();
        external_url = external_url.trim_left_matches("/");
        external_url = external_url.trim_right_matches("/");
        external_url.replace("/", ".")
    }
}
