//! Middleware interfaces for the pathfinder project
//!

use futures::Future;

use engine::serializer::JsonMessage;
use error::PathfinderError;

/// Type alias for future result type.
pub type MiddlewareFuture = Box<Future<Item=(), Error=PathfinderError> + Sync + Send + 'static>;

/// A trait for types which can be used as middleware during processing a request from a client.
pub trait Middleware: Send + Sync {
    /// Applied transforms and checks to an incoming request. If it failed,
    /// then should return a `PathfinderError` instance.
    fn process_request(&self, message: JsonMessage) -> MiddlewareFuture;
}
