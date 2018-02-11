//! Proxy engine
//!
//! This module is intended for processing incoming requests from clients,
//! handling occurred errors during a work, communicating with a message
//! broker and preparing appropriate responses in the certain format.
//!

pub mod rabbitmq;
pub mod router;
pub mod serializer;

use std::cell::{RefCell};
use std::rc::{Rc};
use std::str::{from_utf8};
use std::vec::Vec;

pub use self::router::{Router, Endpoint, extract_endpoints};
pub use self::serializer::{Serializer, JsonMessage};
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
use tokio::reactor::{Handle};
use tungstenite::{Message};
use uuid::{Uuid};


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
    pub fn handle(&self, message: JsonMessage, transmitter: mpsc::UnboundedSender<Message>, handle: &Handle) -> RabbitMQFuture {
        let message_nested = message.clone();
        let url = message_nested["url"].as_str().unwrap();

        let endpoint = self.router.clone().match_url_or_default(&url);
        let endpoint_link = endpoint.clone();
        let endpoint_publish = endpoint.clone();

        let queue_name = Rc::new(format!("{}", Uuid::new_v4()));
        let queue_name_bind = queue_name.clone();
        let queue_name_response = queue_name.clone();
        let queue_name_consumer = queue_name.clone();
        let queue_name_delete = queue_name.clone();

        let request_headers = self.prepare_request_headers(&message_nested, endpoint.clone());
        let client_future = self.rabbitmq_client.borrow().get_future(handle);

        Box::new(client_future
            // 1. Create a channel
            .and_then(move |client| {
                client.create_confirm_channel(ConfirmSelectOptions::default())
            })

            // 2. Declare a response queue
            .and_then(move |channel| {
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

            // 3. Link the response queue the exchange
            .and_then(move |channel| {
                channel.queue_bind(
                    &queue_name_bind,
                    &endpoint_link.get_response_exchange(),
                    &queue_name_bind,
                    &QueueBindOptions::default(),
                    &FieldTable::new()
                )
                    .map(|_| channel)
            })

             // 4. Publish message into the microservice queue and make ensure that it's delivered
            .and_then(move |channel| {
                let publish_message_options = BasicPublishOptions {
                    mandatory: true,
                    immediate: false,
                    ..Default::default()
                };

                let mut message_headers = FieldTable::new();
                for &(ref key, ref value) in request_headers.clone().iter() {
                    let header_name = key.to_string();
                    let header_value = AMQPValue::LongString(value.to_string());
                    message_headers.insert(header_name, header_value);
                }

                let event_name = message["event-name"].as_str().unwrap_or("null");

                let basic_properties = BasicProperties {
                    content_type: Some("application/json".to_string()),
                    headers: Some(message_headers),                       // Headers for the message
                    delivery_mode: Some(2),                               // Message must be persistent
                    reply_to: Some(queue_name_response.to_string()),      // Response queue
                    correlation_id: Some(event_name.clone().to_string()), // Event name
                    ..Default::default()
                };

                channel.basic_publish(
                    &endpoint_publish.get_request_exchange(),
                    &endpoint_publish.get_microservice(),
                    message["content"].dump().as_bytes(),
                    &publish_message_options,
                    basic_properties
                )
                    .map(|_confirmation| channel)
            })

            // 5. Consume a response message from the queue, that was declared on the 2nd step
            .and_then(move |channel| {
                channel.basic_consume(
                    &queue_name_consumer,
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

            // 6. Prepare a response for a client, serialize and sent via WebSocket transmitter
            .and_then(move |(channel, message)| {
                let raw_data = from_utf8(&message.data).unwrap();
                let json = Rc::new(Box::new(json_parse(raw_data).unwrap()));
                let serializer = Serializer::new();
                let response = serializer.serialize(json.dump()).unwrap();
                transmitter.unbounded_send(response).unwrap();
                channel.basic_ack(message.delivery_tag)
                    .map(move |_| channel)
            })

            // 7. Unbind the response queue from the exchange point
            // TODO: Uncomment this code when my PR for queue_unbind will be merged
            //.and_then(move |channel| {
            //    channel.queue_unbind(
            //        &queue_name_unbind,
            //        &response_exchange_name,
            //        &endpoint_unbind.get_microservice(),
            //        &QueueUnbindOptions::default(),
            //        &FieldTable::new()
            //    )
            //        .map(|_| channel)
            //})

            // 8. Delete the response queue
            .and_then(move |channel| {
                let queue_delete_options = QueueDeleteOptions {
                    if_unused: false,
                    if_empty: false,
                    ..Default::default()
                };

                channel.queue_delete(&queue_name_delete, &queue_delete_options)
                    .map(|_| channel)
            })

            // 9. Close the channel
            .and_then(move |channel| {
                channel.close(200, "Close the channel.")
            })

            .then(move |result| {
                match result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!("Error in RabbitMQ client. Reason -> {}", err);
                        let message = String::from("The request wasn't processed. Please, try once again.");
                        Err(PathfinderError::MessageBrokerError(message))
                    }
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
    pub fn serialize_message(&self, json: JsonMessage) -> Message {
        let serializer = Serializer::new();
        serializer.serialize(json.dump()).unwrap()
    }

    /// Deserialize a message into JSON object.
    pub fn deserialize_message(&self, message: &Message) -> Result<JsonMessage> {
        let serializer = Serializer::new();
        serializer.deserialize(message)
    }

    /// Prepares a list of key-value pairs for headers in message.
    fn prepare_request_headers(&self, json: &JsonMessage, endpoint: Rc<Box<Endpoint>>) -> Box<Vec<(String, String)>> {
        Box::new(vec![
            (String::from("Microservice-Name"), endpoint.get_microservice()),
            (String::from("Request-URI"), endpoint.get_url()),
            (String::from("Event-Name"), json["event-name"].as_str().unwrap_or("null").to_string()),
            (String::from("Token"), json["token"].as_str().unwrap_or("null").to_string())
        ])
    }
}
