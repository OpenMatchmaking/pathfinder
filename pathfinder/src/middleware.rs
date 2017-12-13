use std::collections::{HashMap};

use super::error::{PathfinderError};

use cli::{CliOptions};
use futures::{Future};
use futures::future::{lazy};
use futures::sync::{mpsc};
use tokio_core::reactor::{Handle};
use tungstenite::protocol::{Message};


pub type Headers = HashMap<String, Box<[u8]>>;
pub type TungsteniteSender = mpsc::UnboundedSender<Message>;
pub type MiddlewareFuture = Box<Future<Item=(), Error=PathfinderError> + 'static>;


pub trait Middleware {
    /// Applied transforms and checks to an incoming request. If it failed,
    /// then should return an PathfinderError instance.
    fn process_request(&self, headers: &Headers, handle: &Handle) -> MiddlewareFuture;
}


/// Default class which is used for reverse proxy without an authentication
/// header validation process.
pub struct EmptyMiddleware;


impl EmptyMiddleware {
    pub fn new(_cli: &CliOptions) -> EmptyMiddleware {
        EmptyMiddleware {}
    }
}


impl Middleware for EmptyMiddleware {
    fn process_request(&self, _headers: &Headers, _handle: &Handle) -> MiddlewareFuture {
        Box::new( lazy(move || { Ok(()) }) )
    }
}
