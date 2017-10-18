use std::cell::{RefCell};
use std::collections::{HashMap};
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr};
use std::rc::{Rc};

use super::router::{Router};

use futures::sync::{mpsc};
use json::{parse as parse_json, JsonValue};
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
        transmitter.unbounded_send(message.clone()).unwrap();
    }

    fn serialize_message(&self) {

    }

    fn deserialize_message(&self) {

    }

    fn prepare_request(&self, message: &Message){

    }

    fn prepare_response(&self, json: Box<JsonValue>) {
    }
}
