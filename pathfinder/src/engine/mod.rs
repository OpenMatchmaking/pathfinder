//! Proxy engine
//!
//! This module is intended for processing incoming requests from clients,
//! handling occurred errors during a work, communicating with a message
//! broker and preparing appropriate responses in the certain format.
//!

#[macro_use]
pub mod engine_macro;
pub mod router;
pub mod serializer;

pub use self::router::{Router, Endpoint, extract_endpoints};
pub use self::serializer::{Serializer};

use std::cell::{RefCell};
use std::collections::{HashMap};
use std::net::{SocketAddr};
use std::rc::{Rc};

use super::error::{Result};

use futures::sync::{mpsc};
use json::{JsonValue};
use tungstenite::{Message};
use uuid::{Uuid};


/// Type alias for dictionary with `SocketAddr` as a key and `UnboundedSender<Message>` as a value.
pub type ActiveConnections = Rc<RefCell<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>;


/// Proxy engine for processing messages, handling errors and communicating with a message broker.
pub struct Engine {
    router: Box<Router>
}


impl Engine {
    /// Returns a new instance of `Engine`.
    pub fn new(router: Box<Router>) -> Engine {
        Engine {
            router: router
        }
    }

    /// Main handler for generating response per each incoming request.
    pub fn handle(&self, message: Box<JsonValue>, client: &SocketAddr, connections: &ActiveConnections) {
        let transmitter = &connections.borrow_mut()[&client];
        let request = self.prepare_request(message);

        println!("{}", request);
        let response = self.prepare_response(request);
        transmitter.unbounded_send(response).unwrap();
    }

    /// Transforms an error (which is a string) into JSON object in the special format.
    pub fn wrap_an_error(&self, err: &str) -> Message {
        let json_error_message = object!("details" => err);
        let serializer = Serializer::new();
        serializer.serialize(json_error_message.dump()).unwrap()
    }

    /// Serialize a JSON object into message.
    pub fn serialize_message(&self, json: Box<JsonValue>) -> Message {
        let serializer = Serializer::new();
        serializer.serialize(json.dump()).unwrap()
    }

    /// Deserialize a message into JSON object.
    pub fn deserialize_message(&self, message: &Message) -> Result<Box<JsonValue>> {
        let serializer = Serializer::new();
        serializer.deserialize(message)
    }

    /// Converts a JSON object into a message for a message broker.
    fn prepare_request(&self, json: Box<JsonValue>) -> Box<JsonValue> {
        let mut request = Box::new(object!{
            "headers" => object!{},
            "content" => object!{}
        });

        self.generate_request_headers(&mut request, json);
        request
    }

    /// Converts a JSON object into the `tungstenite::Message` message.
    fn prepare_response(&self, json: Box<JsonValue>) -> Message {
        self.serialize_message(json)
    }

    /// Transforms a request to the appropriate format for a further processing by a microservice.
    fn generate_request_headers(&self, request: &mut Box<JsonValue>, from_json: Box<JsonValue>) {
        request["headers"]["request-id"] = format!("{}", Uuid::new_v4()).into();

        let url = from_json["url"].as_str().unwrap();
        match self.router.match_url(url) {
            Ok(endpoint) => {
                request["headers"]["microservice-name"] = endpoint.get_microservice().into();
                request["headers"]["request-url"] = endpoint.get_url().into();
            },
            Err(_) => {
                request["headers"]["microservice-name"] = self.convert_url_into_microservice(url).into();
                request["headers"]["request-url"] = url.into();
            }
        }
    }

    /// Converts a URL to the certain microservice name, so that it will be used as a queue/topic name futher.
    fn convert_url_into_microservice(&self, url: &str) -> String {
        let mut external_url = url.clone();
        external_url = external_url.trim_left_matches("/");
        external_url = external_url.trim_right_matches("/");
        external_url.replace("/", ".")
    }
}
