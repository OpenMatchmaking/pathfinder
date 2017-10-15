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
        let server = socket.incoming().for_each(|(stream, addr)| {
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
                    // Per each message
                    ws_stream.for_each(move |message: Message| {
                        // Convert an incoming message into JSON
                        let text_message = try!(message.into_text());
                        let json_message = try!(proxy_inner.decode_message(text_message.as_str()));

                        // Convert the specified API url to a Kafka topic
                        println!("Message = {}", json_message);
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

    fn decode_message(&self, message: &str) -> BaseResult<JsonValue, Error> {
        match parse_json(message) {
            Ok(message) => Ok(message),
            Err(err) => Err(Error::new(ErrorKind::InvalidData, err))
        }
    }
}
