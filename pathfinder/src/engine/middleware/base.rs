//! Middleware interfaces for the pathfinder project
//!

use std::collections::HashMap;
use std::sync::Arc;

use futures::Future;

use crate::engine::serializer::JsonMessage;
use crate::error::PathfinderError;
use crate::rabbitmq::RabbitMQClient;

/// Type alias for dictionary with custom user headers
pub type CustomUserHeaders = HashMap<String, String>;

/// Type alias for future result type.
pub type MiddlewareFuture = Box<Future<Item=CustomUserHeaders, Error=PathfinderError> + Sync + Send + 'static>;

/// A trait for types that could be used as middleware
/// during processing a request from a client.
pub trait Middleware: Send + Sync {
    /// Applied transforms and checks to an incoming request. If it failed,
    /// then should return a `PathfinderError` instance.
    fn process_request(&self, message: JsonMessage, rabbitmq_client: Arc<RabbitMQClient>) -> MiddlewareFuture;
}
