//! The middleware implementation with JSON Web Token support.
//!

use std::collections::HashMap;
use std::str::from_utf8;
use std::sync::Arc;
use std::vec::Vec;

use futures::future::{lazy, Future};
use futures::Stream;
use json::{object, parse as parse_json};
use lapin_futures_rustls::lapin::channel::{
    BasicConsumeOptions, BasicProperties, BasicPublishOptions, QueueBindOptions,
    QueueDeclareOptions, QueueDeleteOptions, QueueUnbindOptions,
};
use lapin_futures_rustls::lapin::types::{AMQPValue, FieldTable};
use log::{error, info, warn};
use uuid::Uuid;

use crate::error::PathfinderError;
use crate::engine::{RESPONSE_EXCHANGE};
use crate::engine::middleware::{
    TOKEN_VERIFY_ROUTING_KEY,
    TOKEN_VERIFY_EXCHANGE,
    TOKEN_USER_PROFILE_ROUTING_KEY,
    TOKEN_USER_PROFILE_EXCHANGE
};
use crate::engine::middleware::base::{Middleware, MiddlewareFuture, CustomUserHeaders};
use crate::engine::middleware::utils::get_permissions;
use crate::engine::options::RpcOptions;
use crate::engine::serializer::JsonMessage;
use crate::rabbitmq::RabbitMQContext;

/// A middleware class, that will check a JSON Web Token in WebSocket message.
/// If token wasn't specified or it's invalid returns a `PathfinderError` object.
pub struct JwtTokenMiddleware;

impl JwtTokenMiddleware {
    /// Returns a new instance of `JwtTokenMiddleware` structure.
    pub fn new() -> JwtTokenMiddleware {
        JwtTokenMiddleware {}
    }

    /// Performs a request to Auth/Auth microservice with the taken token
    /// that must be verified before doing any actions later.
    fn verify_token(&self, message: JsonMessage, token: String, rabbitmq_context: Arc<RabbitMQContext>)
        -> impl Future<Item=(), Error=PathfinderError> + Sync + Send + 'static
    {
        let access_token = token.clone();
        let options = Arc::new(RpcOptions::default()
            .with_message(message.clone())
            .with_queue_name(Arc::new(format!("{}", Uuid::new_v4())))
        );
        let rabbitmq_context_local = rabbitmq_context.clone();
        let publish_channel = rabbitmq_context_local.get_publish_channel();
        let consume_channel = rabbitmq_context_local.get_consume_channel();

        let queue_name = options.get_queue_name().unwrap().clone();
        let queue_declare_options = QueueDeclareOptions {
            passive: false,
            durable: true,
            exclusive: true,
            auto_delete: false,
            ..Default::default()
        };

        // 1. Declare a response queue
        consume_channel
            .queue_declare(&queue_name, queue_declare_options, FieldTable::new())
            .map(move |queue| (publish_channel, consume_channel, queue, options))
        // 2. Link the response queue the exchange
        .and_then(move |(publish_channel, consume_channel, queue, options)| {
            let queue_name = options.get_queue_name().unwrap().clone();
            let routing_key = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_bind(
                    &queue_name,
                    RESPONSE_EXCHANGE.clone(),
                    &routing_key,
                    QueueBindOptions::default(),
                    FieldTable::new()
                )
                .map(move |_| (publish_channel, consume_channel, queue, options))
        })
        // 3. Publish message into the microservice queue and make ensure that it's delivered
        .and_then(move |(publish_channel, consume_channel, queue, options)| {
            let publish_message_options = BasicPublishOptions {
                mandatory: true,
                immediate: false,
                ..Default::default()
            };

            let request_headers: Vec<(String, String)> = vec![
                (String::from("routing_key"), String::from("auth.token.verify")),
                (String::from("request_url"), String::from("/auth/api/token/verify")),
            ];
            let mut message_headers = FieldTable::new();
            for &(ref key, ref value) in request_headers.iter() {
                let header_name = key.to_string();
                let header_value = AMQPValue::LongString(value.to_string());
                message_headers.insert(header_name, header_value);
            }

            let message = options.get_message().unwrap().clone();
            let queue_name_response = options.get_queue_name().unwrap().clone();
            let event_name = message["event-name"].as_str().unwrap_or("null");
            let request_body = object!{ "access_token" => access_token };
            let basic_properties = BasicProperties::default()
                .with_content_type("application/json".to_string())    // Content type
                .with_headers(message_headers)                        // Headers for the message
                .with_delivery_mode(2)                                // Message must be persistent
                .with_reply_to(queue_name_response.to_string())       // Response queue
                .with_correlation_id(event_name.clone().to_string()); // Event name

            publish_channel
                .basic_publish(
                    TOKEN_VERIFY_EXCHANGE.clone(),
                    TOKEN_VERIFY_ROUTING_KEY.clone(),
                    request_body.dump().as_bytes().to_vec(),
                    publish_message_options,
                    basic_properties
                )
                .map(move |confirmation| {
                    match confirmation {
                        Some(_) => info!("Publish for verifying JWT got confirmation."),
                        None => warn!("Request for verifying JWT wasn't delivered."),
                    };

                    (publish_channel, consume_channel, queue, options)
                })
        })
        // 4. Consume a response message from the queue, that was declared on the 2nd step
        .and_then(move |(publish_channel, consume_channel, queue, options)| {
            consume_channel
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
                        .map(move |(message, _)| (publish_channel, consume_channel, queue, message.unwrap(), options))
                })
        })
        // 5. Prepare a response for a client, serialize and pass to the next processing stage
        .and_then(move |(publish_channel, consume_channel, queue, message, options)| {
            let raw_data = from_utf8(&message.data).unwrap();
            let json = parse_json(raw_data).unwrap();

            consume_channel
                .basic_ack(message.delivery_tag, false)
                .map(move |_confirmation| (publish_channel, consume_channel, queue, options, json))
        })
        // 6. Unbind the response queue from the exchange point
        .and_then(move |(publish_channel, consume_channel, _queue, options, json)| {
            let queue_name = options.get_queue_name().unwrap().clone();
            let routing_key = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_unbind(
                    &queue_name,
                    RESPONSE_EXCHANGE.clone(),
                    &routing_key,
                    QueueUnbindOptions::default(),
                    FieldTable::new(),
                )
                .map(move |_| (publish_channel, consume_channel, options, json))
        })
        // 7. Delete the response queue
        .and_then(move |(_publish_channel, consume_channel, options, json)| {
            let queue_delete_options = QueueDeleteOptions {
                if_unused: false,
                if_empty: false,
                ..Default::default()
            };
            let queue_name = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_delete(&queue_name, queue_delete_options)
                .map(move |_| json)
        })
        // 8. Prepare the response for the client
        .then(move |result| match result {
            Ok(json) => {
                let has_errors = !json["error"].is_null();
                if has_errors {
                    let errors = json["error"].clone();
                    return Err(PathfinderError::MicroserviceError(errors))
                };

                let is_valid_response = !json["content"].is_null();
                let is_valid_token = json["content"]["is_valid"].as_bool().unwrap();
                match is_valid_response && is_valid_token {
                    true => Ok(()),
                    false => {
                        let message = String::from("Token is invalid.");
                        Err(PathfinderError::AuthenticationError(message))
                    }
                }
            },
            Err(err) => {
                error!("Error in RabbitMQ client. Reason: {}", err);
                let message = String::from("The request wasn't processed. Please, try once again.");
                Err(PathfinderError::MessageBrokerError(message))
            }
        })
    }

    /// Performs a request to Auth/Auth microservice with the taken token
    /// that will be used for getting a list of permissions to other resources.
    fn get_headers(&self, message: JsonMessage, token: String, rabbitmq_context: Arc<RabbitMQContext>)
        -> impl Future<Item=CustomUserHeaders, Error=PathfinderError> + Sync + Send + 'static
    {
        let access_token = token.clone();
        let options = Arc::new(RpcOptions::default()
            .with_message(message.clone())
            .with_queue_name(Arc::new(format!("{}", Uuid::new_v4())))
        );
        let rabbitmq_context_local = rabbitmq_context.clone();
        let publish_channel = rabbitmq_context_local.get_publish_channel();
        let consume_channel = rabbitmq_context_local.get_consume_channel();

        let queue_name = options.get_queue_name().unwrap().clone();
        let queue_declare_options = QueueDeclareOptions {
            passive: false,
            durable: true,
            exclusive: true,
            auto_delete: false,
            ..Default::default()
        };

        // 1. Declare a response queue
        consume_channel
            .queue_declare(&queue_name, queue_declare_options, FieldTable::new())
            .map(move |queue| (publish_channel, consume_channel, queue, options))
        // 2. Link the response queue the exchange
        .and_then(move |(publish_channel, consume_channel, queue, options)| {
            let queue_name = options.get_queue_name().unwrap().clone();
            let routing_key = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_bind(
                    &queue_name,
                    RESPONSE_EXCHANGE.clone(),
                    &routing_key,
                    QueueBindOptions::default(),
                    FieldTable::new()
                )
                .map(move |_| (publish_channel, consume_channel, queue, options))
        })
        // 3. Publish message into the microservice queue and make ensure that it's delivered
        .and_then(move |(publish_channel, consume_channel, queue, options)| {
            let publish_message_options = BasicPublishOptions {
                mandatory: true,
                immediate: false,
                ..Default::default()
            };

            let request_headers: Vec<(String, String)> = vec![
                (String::from("microservice_name"), String::from("microservice-auth")),
                (String::from("request_url"), String::from("/auth/api/users/profile")),
            ];
            let mut message_headers = FieldTable::new();
            for &(ref key, ref value) in request_headers.iter() {
                let header_name = key.to_string();
                let header_value = AMQPValue::LongString(value.to_string());
                message_headers.insert(header_name, header_value);
            }

            let message = options.get_message().unwrap().clone();
            let queue_name_response = options.get_queue_name().unwrap().clone();
            let event_name = message["event-name"].as_str().unwrap_or("null");
            let request_body = object!{ "access_token" => access_token };
            let basic_properties = BasicProperties::default()
                .with_content_type("application/json".to_string())    // Content type
                .with_headers(message_headers)                        // Headers for the message
                .with_delivery_mode(2)                                // Message must be persistent
                .with_reply_to(queue_name_response.to_string())       // Response queue
                .with_correlation_id(event_name.clone().to_string()); // Event name

            publish_channel
                .basic_publish(
                    TOKEN_USER_PROFILE_EXCHANGE.clone(),
                    TOKEN_USER_PROFILE_ROUTING_KEY.clone(),
                    request_body.dump().as_bytes().to_vec(),
                    publish_message_options,
                    basic_properties
                )
                .map(move |confirmation| {
                    match confirmation {
                        Some(_) => info!("Publish for getting headers got confirmation."),
                        None => warn!("Request for getting headers wasn't delivered."),
                    };

                    (publish_channel, consume_channel, queue, options)
                })
        })
        // 4. Consume a response message from the queue, that was declared on the 2nd step
        .and_then(move |(publish_channel, consume_channel, queue, options)| {
            consume_channel
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
                        .map(move |(message, _)| (publish_channel, consume_channel, queue, message.unwrap(), options))
                })
        })
        // 5. Prepare a response for a client, serialize and pass to the next processing stage
        .and_then(move |(publish_channel, consume_channel, queue, message, options)| {
            let raw_data = from_utf8(&message.data).unwrap();
            let json = parse_json(raw_data).unwrap();

            consume_channel
                .basic_ack(message.delivery_tag, false)
                .map(move |_confirmation| (publish_channel, consume_channel, queue, options, json))
        })
        // 6. Unbind the response queue from the exchange point
        .and_then(move |(publish_channel, consume_channel, _queue, options, json)| {
            let queue_name = options.get_queue_name().unwrap().clone();
            let routing_key = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_unbind(
                    &queue_name,
                    RESPONSE_EXCHANGE.clone(),
                    &routing_key,
                    QueueUnbindOptions::default(),
                    FieldTable::new(),
                )
                .map(move |_| (publish_channel, consume_channel, options, json))
        })
        // 7. Delete the response queue
        .and_then(move |(_publish_channel, consume_channel, options, json)| {
            let queue_delete_options = QueueDeleteOptions {
                if_unused: false,
                if_empty: false,
                ..Default::default()
            };
            let queue_name = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_delete(&queue_name, queue_delete_options)
                .map(move |_| json)
        })
        // 8. Prepare the response for the client
        .then(move |result| match result {
            Ok(json) => {
                let has_errors = !json["error"].is_null();
                if has_errors {
                    let errors = json["error"].clone();
                    return Err(PathfinderError::MicroserviceError(errors))
                };

                let is_valid_response = !json["content"].is_null();
                match is_valid_response {
                    true => {
                        let mut extra_headers: CustomUserHeaders = HashMap::new();
                        extra_headers.insert(String::from("permissions"), get_permissions(&json));
                        Ok(extra_headers)
                    },
                    false => Ok(HashMap::new())
                }
            },
            Err(err) => {
                error!("Error in RabbitMQ client. Reason: {}", err);
                let message = String::from("The request wasn't processed. Please, try once again.");
                Err(PathfinderError::MessageBrokerError(message))
            }
        })
    }
}

impl Middleware for JwtTokenMiddleware {
    fn process_request(&self, message: JsonMessage, rabbitmq_context: Arc<RabbitMQContext>) -> MiddlewareFuture {
        // Extract a token from a JSON object
        let token = match message["token"].as_str() {
            Some(token) => String::from(token),
            _ => {
                return Box::new(lazy(move || {
                    let message = String::from("The `token` field must be specified.");
                    Err(PathfinderError::AuthenticationError(message))
                }))
            }
        };

        // Verify the passed JSON Web Token and extract permissions
        let verify_token_future = self.verify_token(message.clone(),token.clone(), rabbitmq_context.clone());
        let get_headers_future = self.get_headers(message.clone(),token.clone(), rabbitmq_context.clone());
        Box::new(verify_token_future.and_then(move |_| get_headers_future))
    }
}
