/// This module provides default futures that can be used for
/// sending request to other microservices by proxy engine.
///

use std::collections::HashMap;
use std::str::from_utf8;
use std::sync::Arc;

use futures::future::{Future};
use futures::Stream;
use json::parse as json_parse;
use lapin_futures_rustls::lapin::channel::{
    BasicConsumeOptions, BasicProperties, BasicPublishOptions, QueueBindOptions,
    QueueDeclareOptions, QueueDeleteOptions, QueueUnbindOptions,
};
use lapin_futures_rustls::lapin::types::{AMQPValue, FieldTable};
use log::error;

use crate::error::PathfinderError;
use crate::rabbitmq::{RabbitMQContext};
use crate::engine::MessageSender;
use crate::engine::options::RpcOptions;
use crate::engine::serializer::Serializer;

/// Simple future that sends a RPC request to the certain microservice,
/// consumes from a response from a separate queue and then returns a
/// response to the caller via transmitter.
pub fn rpc_request_future(
    transmitter: MessageSender,
    rabbitmq_context: Arc<RabbitMQContext>,
    options: Arc<RpcOptions>,
    headers: HashMap<String, String>
) -> Box<Future<Item=(), Error=PathfinderError> + Send + Sync + 'static> {
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

    Box::new(
        // 1. Declare a response queue
        consume_channel
            .queue_declare(&queue_name, queue_declare_options, FieldTable::new())
            .map(move |queue| (publish_channel, consume_channel, queue, options))
        // 2. Link the response queue the exchange
        .and_then(move |(publish_channel, consume_channel, queue, options)| {
            let queue_name = options.get_queue_name().unwrap().clone();
            let endpoint = options.get_endpoint().unwrap().clone();
            let routing_key = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_bind(
                    &queue_name,
                    &endpoint.get_response_exchange(),
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

            let mut message_headers = FieldTable::new();
            for (key, value) in headers.clone().iter() {
                let header_name = key.clone();
                let header_value = AMQPValue::LongString(value.clone());
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

            publish_channel
                .basic_publish(
                    &endpoint.get_request_exchange(),
                    &endpoint.get_routing_key(),
                    message["content"].dump().as_bytes().to_vec(),
                    publish_message_options,
                    basic_properties
                )
                .map(move |_confirmation| (publish_channel, consume_channel, queue, options))
        })
        // 4. Consume a response message from the queue, that was declared on the 1st step
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
        // 5. Prepare a response for a client, serialize and sent via WebSocket transmitter
        .and_then(move |(publish_channel, consume_channel, queue, message, options)| {
            let raw_data = from_utf8(&message.data).unwrap();
            let json = Arc::new(Box::new(json_parse(raw_data).unwrap()));
            let serializer = Serializer::new();
            let response = serializer.serialize(json.dump()).unwrap();
            let transmitter_local = transmitter.clone();
            transmitter_local.unbounded_send(response).unwrap_or(());

            consume_channel
                .basic_ack(message.delivery_tag, false)
                .map(move |_confirmation| (publish_channel, consume_channel, queue, options))
        })
        // 6. Unbind the response queue from the exchange point
        .and_then(move |(publish_channel, consume_channel, _queue, options)| {
            let queue_name = options.get_queue_name().unwrap().clone();
            let routing_key = options.get_queue_name().unwrap().clone();
            let endpoint = options.get_endpoint().unwrap().clone();

            consume_channel
                .queue_unbind(
                    &queue_name,
                    &endpoint.get_response_exchange(),
                    &routing_key,
                    QueueUnbindOptions::default(),
                    FieldTable::new(),
                )
                .map(move |_| (publish_channel, consume_channel, options))
        })
        // 7. Delete the response queue
        .and_then(move |(_publish_channel, consume_channel, options)| {
            let queue_delete_options = QueueDeleteOptions {
                if_unused: false,
                if_empty: false,
                ..Default::default()
            };
            let queue_name = options.get_queue_name().unwrap().clone();

            consume_channel
                .queue_delete(&queue_name, queue_delete_options)
                .map(move |_| ())
        })
        // 8. Returns the result to the caller as future
        .then(move |result| match result {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("Error in RabbitMQ client. Reason: {}", err);
                let message = String::from("The request wasn't processed. Please, try once again.");
                Err(PathfinderError::MessageBrokerError(message))
            }
        })
    )
}
