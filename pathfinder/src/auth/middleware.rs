//! Middleware interfaces for the pathfinder project
//!

use super::super::error::{PathfinderError};
use super::super::engine::serializer::{JsonMessage};

use cli::{CliOptions};
use futures::{Future};
use futures::future::{lazy};


/// Type alias for future result type.
pub type MiddlewareFuture = Box<Future<Item=(), Error=PathfinderError> + 'static>;


/// A trait for types which can be used as middleware during processing a request from a client.
pub trait Middleware {
    /// Applied transforms and checks to an incoming request. If it failed,
    /// then should return a `PathfinderError` instance.
    fn process_request(&self, message: JsonMessage) -> MiddlewareFuture;
}


/// Default struct which is used for reverse proxy without an authentication
/// layer.
pub struct EmptyMiddleware;


impl EmptyMiddleware {
    pub fn new(_cli: &CliOptions) -> EmptyMiddleware {
        EmptyMiddleware {}
    }
}


impl Middleware for EmptyMiddleware {
    /// Returns an empty future which is doesn't doing anything.
    fn process_request(&self, _message: JsonMessage) -> MiddlewareFuture {
        Box::new(lazy(move || Ok(())))
    }
}
