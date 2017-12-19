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


type ActiveConnections = Rc<RefCell<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>;


pub struct Engine {
    router: Box<Router>
}


impl Engine {
    pub fn new(router: Box<Router>) -> Engine {
        Engine {
            router: router
        }
    }

    pub fn handle(&self, message: &Message, client: &SocketAddr, connections: &ActiveConnections) {
        let transmitter = &connections.borrow_mut()[&client];

        let request = match self.prepare_request(message) {
            Ok(json_object) => json_object,
            Err(err) => {
                let formatted_error = format!("{}", err);
                let error_message = self.wrap_an_error(formatted_error.as_str());
                transmitter.unbounded_send(error_message).unwrap();
                return
            }
        };

        println!("{}", request);
        let response = self.prepare_response(request);
        transmitter.unbounded_send(response).unwrap();
    }

    pub fn wrap_an_error(&self, err: &str) -> Message {
        let json_error_message = object!("details" => err);
        let serializer = Serializer::new();
        serializer.serialize(json_error_message.dump()).unwrap()
    }

    pub fn serialize_message(&self, json: Box<JsonValue>) -> Message {
        let serializer = Serializer::new();
        serializer.serialize(json.dump()).unwrap()
    }

    pub fn deserialize_message(&self, message: &Message) -> Result<Box<JsonValue>> {
        let serializer = Serializer::new();
        serializer.deserialize(message)
    }

    fn prepare_request(&self, message: &Message) -> Result<Box<JsonValue>> {
        let json = try!(self.deserialize_message(message));
        let mut request = Box::new(object!{
            "headers" => object!{},
            "content" => object!{}
        });

        self.generate_request_headers(&mut request, json);
        Ok(request)
    }

    fn prepare_response(&self, json: Box<JsonValue>) -> Message {
        self.serialize_message(json)
    }

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

    fn convert_url_into_microservice(&self, url: &str) -> String {
        let mut external_url = url.clone();
        external_url = external_url.trim_left_matches("/");
        external_url = external_url.trim_right_matches("/");
        external_url.replace("/", ".")
    }
}
