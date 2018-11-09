//! Proxy engine
//!
//! This module is intended for processing incoming requests from clients,
//! handling occurred errors during a work, communicating with a message
//! broker and preparing appropriate responses in the certain format.
//!

use std::collections::HashMap;
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
use super::super::error::{Result, PathfinderError};
use super::super::rabbitmq::RabbitMQClient;
use engine::middleware::{EmptyMiddleware, JwtTokenMiddleware, Middleware, MiddlewareFuture};
use engine::router::{extract_endpoints, ReadOnlyEndpoint, Router};
use engine::options::RpcOptions;
use engine::serializer::{JsonMessage, Serializer};
use engine::utils::{deserialize_message};

/// Alias type for msps sender.
pub type MessageSender = Arc<mpsc::UnboundedSender<Message>>;
/// Alias for generic future that can be returned from internal proxy engine
/// handlers and middlewares.
pub type EngineFuture = Box<Future<Item=(), Error=PathfinderError> + Send + Sync + 'static>;

/// Proxy engine for processing messages, handling errors and communicating
/// with a message broker.
pub struct Engine {
    router: Arc<Router>,
    middlewares: Arc<HashMap<String, Arc<Box<Middleware>>>>
}

impl Engine {
    /// Returns a new instance of `Engine`.
    pub fn new(cli: &CliOptions) -> Engine {
        let config = get_config(&cli.config);
        let endpoints = extract_endpoints(config);
        let router = Router::new(endpoints);
        let middlewares_list: Vec<(&str, Box<Middleware>)> = vec![
            ("jwt", Box::new(JwtTokenMiddleware::new())),
            ("empty", Box::new(EmptyMiddleware::new())),
        ];
        let middlewares = middlewares_list
            .into_iter()
            .map(|(key, middleware)| (String::from(key), Arc::new(middleware)))
            .collect();

        Engine {
            router: Arc::new(router),
            middlewares: Arc::new(middlewares),
        }
    }

    pub fn process_request(&self, message: Message, transmitter: MessageSender, rabbitmq_client: Arc<RabbitMQClient>) -> EngineFuture {
        // 1. Deserialize message into JSON
        let json_message = match deserialize_message(&message) {
            Ok(json_message) => json_message,
            Err(error) => return Box::new(lazy(move || Err(error)))
        };

        // 2. Finding an endpoint in according to the URL in the message body
        let url = json_message["url"].as_str().unwrap();
        let endpoint = match self.get_endpoint(url) {
            Ok(endpoint) => endpoint.clone(),
            Err(error) => return Box::new(lazy(move || Err(error)))
        };

        // 3. Instantiate futures that will be processing client credentials and a request
        let rpc_options = Arc::new(RpcOptions::default()
            .with_endpoint(endpoint.clone())
            .with_message(json_message.clone())
            .with_queue_name(Arc::new(format!("{}", Uuid::new_v4())))
        );
        let middleware_future = self.get_middleware_future(rabbitmq_client.clone(), rpc_options.clone());
        let rabbitmq_future = self.rpc_request(rabbitmq_client.clone(), transmitter.clone(), rpc_options.clone());
        Box::new(middleware_future.and_then(move |_| rabbitmq_future))
    }

    /// Returns an endpoint based on specified URL.
    fn get_endpoint(&self, url: &str) -> Result<ReadOnlyEndpoint> {
        let router = self.router.clone();
        router.match_url(&url)
    }

    /// Returns a middleware for processing client credentials.
    fn get_middleware_future(&self, rabbitmq_client: Arc<RabbitMQClient>, options: Arc<RpcOptions>) -> MiddlewareFuture {
        let endpoint = options.get_endpoint().unwrap().clone();
        let middleware = self.get_middleware_by_endpoint(endpoint);

        let json_message = options.get_message().unwrap().clone();
        let rabbitmq_client_local = rabbitmq_client.clone();
        middleware.process_request(json_message, rabbitmq_client_local)
    }

    /// Return a middleware that matches to the passed endpoint
    fn get_middleware_by_endpoint(&self, endpoint: ReadOnlyEndpoint) -> Arc<Box<Middleware>> {
        match endpoint.is_token_required() {
            true => self.middlewares.clone()["jwt"].clone(),
            false => self.middlewares.clone()["empty"].clone()
        }
    }

    /// Main handler for generating a response per each incoming request.
    fn rpc_request(&self, rabbitmq_client: Arc<RabbitMQClient>, transmitter: MessageSender, options: Arc<RpcOptions>) -> EngineFuture {
        let message = options.get_message().unwrap().clone();
        let endpoint = options.get_endpoint().unwrap().clone();
        let request_headers = self.prepare_request_headers(&message, endpoint);
        let rabbitmq_client_local = rabbitmq_client.clone();

        Box::new(
            // 1. Create a channel
            rabbitmq_client_local.get_channel()
            // 2. Declare a response queue
            .and_then(move |channel| {
                let queue_name = options.get_queue_name().unwrap().clone();
                let queue_declare_options = QueueDeclareOptions {
                    passive: false,
                    durable: true,
                    exclusive: true,
                    auto_delete: false,
                    ..Default::default()
                };

                channel
                    .queue_declare(&queue_name, queue_declare_options, FieldTable::new())
                    .map(move |queue| (channel, queue, options))
            })
            // 3. Link the response queue the exchange
            .and_then(move |(channel, queue, options)| {
                let queue_name = options.get_queue_name().unwrap().clone();
                let endpoint = options.get_endpoint().unwrap().clone();
                let routing_key = options.get_queue_name().unwrap().clone();

                channel
                    .queue_bind(
                        &queue_name,
                        &endpoint.get_response_exchange(),
                        &routing_key,
                        QueueBindOptions::default(),
                        FieldTable::new()
                    )
                    .map(move |_| (channel, queue, options))
            })
            // 4. Publish message into the microservice queue and make ensure that it's delivered
            .and_then(move |(channel, queue, options)| {
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

                let endpoint = options.get_endpoint().unwrap().clone();
                let message = options.get_message().unwrap().clone();
                let queue_name_response = options.get_queue_name().unwrap().clone();
                let event_name = message["event-name"].as_str().unwrap_or("null");
                let basic_properties = BasicProperties::default()
                    .with_content_type("application/json".to_string())    // Content type
                    .with_headers(message_headers)                        // Headers for the message
                    .with_delivery_mode(2)                                // Message must be persistent
                    .with_reply_to(queue_name_response.to_string())       // Response queue
                    .with_correlation_id(event_name.clone().to_string()); // Event name

                channel
                    .basic_publish(
                        &endpoint.get_request_exchange(),
                        &endpoint.get_microservice(),
                        message["content"].dump().as_bytes().to_vec(),
                        publish_message_options,
                        basic_properties
                    )
                    .map(move |_confirmation| (channel, queue, options))
            })
            // 5. Consume a response message from the queue, that was declared on the 2nd step
            .and_then(move |(channel, queue, options)| {
                channel
                    .basic_consume(
                        &queue,
                        "response_consumer",
                        BasicConsumeOptions::default(),
                        FieldTable::new()
                    )
                    .and_then(move |stream| {
                        stream
                            .take(1)
                            .into_future()
                            .map_err(|(err, _)| err)
                            .map(move |(message, _)| (channel, queue, message.unwrap(), options))
                    })
            })
            // 6. Prepare a response for a client, serialize and sent via WebSocket transmitter
            .and_then(move |(channel, queue, message, options)| {
                let raw_data = from_utf8(&message.data).unwrap();
                let json = Arc::new(Box::new(json_parse(raw_data).unwrap()));
                let serializer = Serializer::new();
                let response = serializer.serialize(json.dump()).unwrap();
                let transmitter_local = transmitter.clone();
                transmitter_local.unbounded_send(response).unwrap();

                channel
                    .basic_ack(message.delivery_tag, false)
                    .map(move |_confirmation| (channel, queue, options))
            })
            // 7. Unbind the response queue from the exchange point
            .and_then(move |(channel, _queue, options)| {
                let queue_name = options.get_queue_name().unwrap().clone();
                let routing_key = options.get_queue_name().unwrap().clone();
                let endpoint = options.get_endpoint().unwrap().clone();

                channel
                    .queue_unbind(
                        &queue_name,
                        &endpoint.get_response_exchange(),
                        &routing_key,
                        QueueUnbindOptions::default(),
                        FieldTable::new(),
                    )
                    .map(move |_| (channel, options))
            })
            // 8. Delete the response queue
            .and_then(move |(channel, options)| {
                let queue_delete_options = QueueDeleteOptions {
                    if_unused: false,
                    if_empty: false,
                    ..Default::default()
                };
                let queue_name = options.get_queue_name().unwrap().clone();

                channel
                    .queue_delete(&queue_name, queue_delete_options)
                    .map(move |_| channel)
            })
            // 9. Close the channel
            .and_then(move |channel| {
                channel.close(200, "Close the channel.")
            })
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
    fn prepare_request_headers(&self, json: &JsonMessage, endpoint: ReadOnlyEndpoint) -> Box<Vec<(String, String)>> {
        Box::new(vec![
            (String::from("microservice_name"), endpoint.get_microservice()),
            (String::from("request_url"), endpoint.get_url()),
            (String::from("permissions"), json["permissions"].as_str().unwrap_or("").to_string()),
            (String::from("user_id"), json["user_id"].as_str().unwrap_or("").to_string())
        ])
    }
}
