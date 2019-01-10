//! Proxy engine
//!
//! This module is intended for processing incoming requests from clients,
//! handling occurred errors during a work, communicating with a message
//! broker and preparing appropriate responses in the certain format.
//!

use std::collections::HashMap;
use std::sync::Arc;

use futures::future::{lazy, Future};
use tungstenite::Message;
use uuid::Uuid;

use crate::cli::CliOptions;
use crate::config::get_config;
use crate::error::{Result, PathfinderError};
use crate::rabbitmq::RabbitMQContext;
use super::middleware::{
    CustomUserHeaders, EmptyMiddleware, JwtTokenMiddleware, Middleware,
    MiddlewareFuture
};
use super::MessageSender;
use super::futures::rpc_request_future;
use super::router::{extract_endpoints, ReadOnlyEndpoint, Router};
use super::options::RpcOptions;
use super::serializer::JsonMessage;
use super::utils::{deserialize_message};

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

    /// Performs deserializing an incoming message into JSON, searching for
    /// a route, applying a middleware and sending a request to microservice
    /// in the certain format.
    pub fn process_request(
        &self,
        message: Message,
        transmitter: MessageSender,
        rabbitmq_context: Arc<RabbitMQContext>
    ) -> Box<Future<Item=(), Error=PathfinderError> + Send + Sync + 'static> {
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
        let default_headers = self.generate_default_headers(&json_message.clone(), endpoint.clone());
        let transmitter_inner = transmitter.clone();
        let rabbitmq_context_inner = rabbitmq_context.clone();
        let rpc_options = Arc::new(RpcOptions::default()
            .with_endpoint(endpoint.clone())
            .with_message(json_message.clone())
            .with_queue_name(Arc::new(format!("{}", Uuid::new_v4()))
        ));

        let middleware_future = self.get_middleware_future(json_message.clone(), endpoint.clone(), rabbitmq_context.clone());
        Box::new(
            middleware_future.and_then(move |custom_headers: CustomUserHeaders| {
                let mut request_headers = default_headers.clone();
                for (key, value) in custom_headers.clone().iter() {
                    let header_name = key.to_string();
                    let header_value = value.to_string();
                    request_headers.insert(header_name, header_value);
                }
                rpc_request_future(
                    transmitter_inner.clone(),
                    rabbitmq_context_inner.clone(),
                    rpc_options.clone(),
                    request_headers.clone()
                )
            })
        )
    }

    /// Returns an endpoint based on specified URL.
    fn get_endpoint(&self, url: &str) -> Result<ReadOnlyEndpoint> {
        let router = self.router.clone();
        router.match_url(&url)
    }

    /// Returns a middleware for processing client credentials.
    fn get_middleware_future(
        &self,
        json_message: JsonMessage,
        endpoint: ReadOnlyEndpoint,
        rabbitmq_context: Arc<RabbitMQContext>
    ) -> MiddlewareFuture {
        let middleware = self.get_middleware_by_endpoint(endpoint);
        let rabbitmq_client_local = rabbitmq_context.clone();
        middleware.process_request(json_message, rabbitmq_context)
    }

    /// Returns a middleware that matches to the passed endpoint
    fn get_middleware_by_endpoint(&self, endpoint: ReadOnlyEndpoint) -> Arc<Box<Middleware>> {
        match endpoint.is_token_required() {
            true => self.middlewares.clone()["jwt"].clone(),
            false => self.middlewares.clone()["empty"].clone()
        }
    }

    /// Generates default headers for the message.
    fn generate_default_headers(&self, json: &JsonMessage, endpoint: ReadOnlyEndpoint) -> HashMap<String, String> {
        [
            (String::from("routing_key"), endpoint.get_routing_key()),
            (String::from("request_url"), endpoint.get_url()),
            (String::from("permissions"), json["permissions"].as_str().unwrap_or("").to_string()),
            (String::from("user_id"), json["user_id"].as_str().unwrap_or("").to_string()),
        ].iter().cloned().collect()
    }
}
