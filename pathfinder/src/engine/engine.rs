//! Proxy engine
//!
//! This module is intended for processing incoming requests from clients,
//! handling occurred errors during a work, communicating with a message
//! broker and preparing appropriate responses in the certain format.
//!

use std::str::from_utf8;
use std::sync::Arc;
use std::vec::Vec;

use futures::future::{lazy, Future};
use futures::sync::mpsc;
use futures::Stream;
use json::parse as json_parse;
use lapin_futures_rustls::lapin::channel::{
    BasicConsumeOptions, BasicProperties, BasicPublishOptions, QueueBindOptions,
    QueueDeclareOptions, QueueDeleteOptions, QueueUnbindOptions,
};
use lapin_futures_rustls::lapin::types::{AMQPValue, FieldTable};
use tungstenite::Message;
use uuid::Uuid;

use super::super::cli::CliOptions;
use super::super::config::get_config;
use super::super::error::PathfinderError;
use super::super::rabbitmq::RabbitMQClient;
use engine::auth::middleware::{EmptyMiddleware, Middleware};
use engine::auth::token::middleware::JwtTokenMiddleware;
use engine::router::{extract_endpoints, ReadOnlyEndpoint, Router};
use engine::serializer::{JsonMessage, Serializer};
use engine::utils::{deserialize_message, wrap_an_error};

/// Alias type for msps sender.
pub type MessageSender = Arc<mpsc::UnboundedSender<Message>>;
/// Alias for generic future that must be returned to the proxy.
pub type GenericFuture = Box<Future<Item=(), Error=()> + Send + Sync + 'static>;
/// Alias for generic future that can be returned from internal proxy engine
/// handlers and middlewares.
pub type EngineFuture = Box<Future<Item=(), Error=PathfinderError> + Send + Sync + 'static>;

/// Proxy engine for processing messages, handling errors and communicating with a message broker.
pub struct Engine {
    router: Arc<Router>,
    middleware: Arc<Box<Middleware>>,
}

impl Engine {
    /// Returns a new instance of `Engine`.
    pub fn new(cli: &CliOptions) -> Engine {
        let config = get_config(&cli.config);
        let endpoints = extract_endpoints(config);
        let router = Router::new(endpoints);
        let middleware: Box<Middleware> = match cli.validate {
            true => Box::new(JwtTokenMiddleware::new(cli)),
            _ => Box::new(EmptyMiddleware::new(cli)),
        };

        Engine {
            router: Arc::new(router),
            middleware: Arc::new(middleware),
        }
    }

    pub fn process_request(
        &self,
        message: Message,
        transmitter: MessageSender,
        rabbitmq_client: Arc<RabbitMQClient>
    ) -> GenericFuture {
        // 1. Deserialize message into JSON
        let transmitter_local = transmitter.clone();
        let json_message = match deserialize_message(&message) {
            Ok(json_message) => json_message,
            Err(error) => {
                let formatted_error = format!("{}", error);
                let error_message = wrap_an_error(formatted_error.as_str());
                transmitter_local.unbounded_send(error_message).unwrap();
                return Box::new(lazy(move || Ok(())));
            }
        };

        // 2. Apply a middleware to each incoming message
        let middleware_local = self.middleware.clone();
        let auth_future = middleware_local.process_request(json_message.clone());

        // 3. Put request into a queue in RabbitMQ and receive the response
        let transmitter_nested = transmitter.clone();
        let transmitter_nested2 = transmitter.clone();
        let rabbitmq_local = rabbitmq_client.clone();
        let rabbitmq_future = self.handle(json_message.clone(), transmitter_nested, rabbitmq_local);

        Box::new(
            auth_future
                .and_then(move |_| rabbitmq_future)
                .map_err(move |error| {
                    let formatted_error = format!("{}", error);
                    let error_message = wrap_an_error(formatted_error.as_str());
                    transmitter_nested2.unbounded_send(error_message).unwrap();
                    ()
                })
        )
    }

    /// TODO: Replace endpoint/queue clones onto a one struct with expected fields
    /// Main handler for generating a response per each incoming request.
    pub fn handle(
        &self,
        message: JsonMessage,
        transmitter: MessageSender,
        rabbitmq_client: Arc<RabbitMQClient>,
    ) -> EngineFuture {
        let message_nested = message.clone();
        let url = message_nested["url"].as_str().unwrap();

        let router_local = self.router.clone();
        let matched_endpoint = router_local.match_url_or_default(&url);
        let endpoint = matched_endpoint.clone();
        let endpoint_link = endpoint.clone();
        let endpoint_publish = endpoint.clone();
        let endpoint_unbind = endpoint.clone();

        let queue_name = Arc::new(format!("{}", Uuid::new_v4()));
        let queue_name_bind = queue_name.clone();
        let queue_name_response = queue_name.clone();
        let queue_name_unbind = queue_name.clone();
        let queue_name_delete = queue_name.clone();

        let request_headers = self.prepare_request_headers(&message_nested, endpoint.clone());
        let rabbitmq_client_local = rabbitmq_client.clone();
        let channel = rabbitmq_client_local.get_channel();

        Box::new(
            // 1. Create a channel
            channel
                // 2. Declare a response queue
                .and_then(move |channel| {
                    let queue_declare_options = QueueDeclareOptions {
                        passive: false,
                        durable: true,
                        exclusive: true,
                        auto_delete: false,
                        ..Default::default()
                    };

                    channel
                        .queue_declare(&queue_name, queue_declare_options, FieldTable::new())
                        .map(move |queue| (channel, queue))
                })
                // 3. Link the response queue the exchange
                .and_then(move |(channel, queue)| {
                    channel
                        .queue_bind(
                            &queue_name_bind,
                            &endpoint_link.get_response_exchange(),
                            &queue_name_bind,
                            QueueBindOptions::default(),
                            FieldTable::new(),
                        ).map(move |_| (channel, queue))
                })
                // 4. Publish message into the microservice queue and make ensure that it's delivered
                .and_then(move |(channel, queue)| {
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

                    let basic_properties = BasicProperties::default()
                        .with_content_type("application/json".to_string())    // Content type
                        .with_headers(message_headers)                        // Headers for the message
                        .with_delivery_mode(2)                                // Message must be persistent
                        .with_reply_to(queue_name_response.to_string())       // Response queue
                        .with_correlation_id(event_name.clone().to_string()); // Event name

                    channel
                        .basic_publish(
                            &endpoint_publish.get_request_exchange(),
                            &endpoint_publish.get_microservice(),
                            message["content"].dump().as_bytes().to_vec(),
                            publish_message_options,
                            basic_properties,
                        ).map(move |_confirmation| (channel, queue))
                })
                // 5. Consume a response message from the queue, that was declared on the 2nd step
                .and_then(move |(channel, queue)| {
                    channel
                        .basic_consume(
                            &queue,
                            "response_consumer",
                            BasicConsumeOptions::default(),
                            FieldTable::new(),
                        ).and_then(move |stream| {
                            stream
                                .take(1)
                                .into_future()
                                .map_err(|(err, _)| err)
                                .map(move |(message, _)| (channel, queue, message.unwrap()))
                        })
                })
                // 6. Prepare a response for a client, serialize and sent via WebSocket transmitter
                .and_then(move |(channel, queue, message)| {
                    let raw_data = from_utf8(&message.data).unwrap();
                    let json = Arc::new(Box::new(json_parse(raw_data).unwrap()));
                    let serializer = Serializer::new();
                    let response = serializer.serialize(json.dump()).unwrap();
                    transmitter.unbounded_send(response).unwrap();
                    channel
                        .basic_ack(message.delivery_tag, false)
                        .map(move |_confirmation| (channel, queue))
                })
                // 7. Unbind the response queue from the exchange point
                .and_then(move |(channel, _queue)| {
                    channel
                        .queue_unbind(
                            &queue_name_unbind,
                            &endpoint_unbind.get_response_exchange(),
                            &endpoint_unbind.get_microservice(),
                            QueueUnbindOptions::default(),
                            FieldTable::new(),
                        ).map(move |_| channel)
                })
                // 8. Delete the response queue
                .and_then(move |channel| {
                    let queue_delete_options = QueueDeleteOptions {
                        if_unused: false,
                        if_empty: false,
                        ..Default::default()
                    };

                    channel
                        .queue_delete(&queue_name_delete, queue_delete_options)
                        .map(move |_| channel)
                })
                // 9. Close the channel
                .and_then(move |channel| channel.close(200, "Close the channel."))
                .then(move |result| match result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!("Error in RabbitMQ client. Reason -> {}", err);
                        let message = String::from("The request wasn't processed. Please, try once again.");
                        Err(PathfinderError::MessageBrokerError(message))
                    }
                })
        )
    }

    /// Prepares a list of key-value pairs for headers in message.
    fn prepare_request_headers(
        &self,
        json: &JsonMessage,
        endpoint: ReadOnlyEndpoint
    ) -> Box<Vec<(String, String)>> {
        Box::new(vec![
            (String::from("microservice_name"), endpoint.get_microservice()),
            (String::from("request_url"), endpoint.get_url()),
            (String::from("permissions"), json["permissions"].as_str().unwrap_or("").to_string()),
            (String::from("user_id"), json["user_id"].as_str().unwrap_or("").to_string())
        ])
    }
}
