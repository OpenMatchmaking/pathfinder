use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::result::{Result as BaseResult};

use super::router::{Router};

use futures::{Future, Sink};
use futures::stream::{Stream};
use json::{parse as parse_json, JsonValue};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::{Core};
use tokio_tungstenite::{accept_async};
use tungstenite::protocol::{Message};


pub struct Proxy {
    router: Box<Router>,
}


impl Proxy {
    pub fn new(router: Box<Router>) -> Proxy {
        Proxy {
            router: router
        }
    }

    pub fn run(&self, address: SocketAddr) {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let socket = TcpListener::bind(&address, &handle).unwrap();
        println!("Listening on: {}", address);

        // The server loop.
        let server = socket.incoming().for_each(|(stream, _addr)| {
            let proxy_inner = self;

            // Handler per each connection.
            accept_async(stream)
                .map_err(|err| {
                    println!("Occurred error during the WebSocket handshake: {}", err);
                    Error::new(ErrorKind::Other, err)
                })
                // Check the Auth header
                .and_then(move |ws_stream| {
                    println!("Checking the user's token");
                    Ok(ws_stream)
                })
                // Process the messages
                .and_then(move |ws_stream| {
                    let (_sink, stream) = ws_stream.split();

                    // Per each message
                    stream.for_each(move |message: Message| {
                        // Convert an incoming message into JSON
                        let text_message = try!(message.into_text());
                        let parsed_json = try!(proxy_inner.decode_message(text_message.as_str()));

                        // Convert the specified API url to a Kafka topic
                        let json_message = try!(proxy_inner.validate_json(parsed_json));
                        let microservice = proxy_inner.match_microservice(json_message);
                        Ok(())
                    })
                    .map_err(|err| {
                        println!("Occurred error during the processing a message: {}", err);
                        Error::new(ErrorKind::Other, err)
                    })
                })
        });

        // Run the server
        core.run(server).unwrap();
    }

    fn decode_message(&self, message: &str) -> BaseResult<Box<JsonValue>, Error> {
        match parse_json(message) {
            Ok(message) => Ok(Box::new(message)),
            Err(err) => Err(Error::new(ErrorKind::InvalidData, err))
        }
    }

    fn validate_json(&self, json: Box<JsonValue>) -> BaseResult<Box<JsonValue>, Error> {
        if !json.has_key("url") {
            return Err(Error::new(ErrorKind::Other, "Key `url` is missing or value is `null`"));
        }

        if json.has_key("matchmaking") {
            return Err(Error::new(ErrorKind::Other, "Key `matchmaking` must be not specified"));
        }

        Ok(json)
    }

    fn match_microservice(&self, json: Box<JsonValue>) -> String {
        let url = json["url"].as_str().unwrap();

        match self.router.match_url(url) {
            Ok(endpoint) => endpoint.get_microservice(),
            _ => {
                let mut external_url = url.clone();
                external_url = external_url.trim_left_matches("/");
                external_url = external_url.trim_right_matches("/");
                external_url.replace("/", ".")
            }
        }
    }
}
