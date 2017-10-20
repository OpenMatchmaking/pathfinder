use std::cell::{RefCell};
use std::collections::{HashMap};
use std::net::{SocketAddr};
use std::rc::{Rc};

use super::error::{Result};
use super::router::{Router};
use super::serializer::{Serializer};

use futures::sync::{mpsc};
use json::{JsonValue};
use tungstenite::{Message};


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
    }

    fn wrap_an_error(&self, err: &str) -> Message {
        let json_error_message = object!("details" => err);
        Message::Text(json_error_message.dump())
    }

    fn serialize_message(&self) {
    }

    fn deserialize_message(&self, message: &Message) -> Result<Box<JsonValue>> {
        let serializer = Serializer::new();
        serializer.deserialize(message)
    }

    fn prepare_request(&self, message: &Message) -> Result<Box<JsonValue>> {
         self.deserialize_message(message)
    }

    fn prepare_response(&self, json: Box<JsonValue>) {
    }
}
