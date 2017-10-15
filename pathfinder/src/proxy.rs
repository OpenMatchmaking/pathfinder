use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use super::router::{Router};

use futures::{Future, Sink};
use futures::stream::{Stream};
use json::{parse as parse_json};
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
            let handle_inner = handle.clone();

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
                // Process the message
                .and_then(move |ws_stream| {
                    ws_stream.for_each(move |message: Message| {
                        // Convert message into text
                        // After it try to parse into JSON
                        // Convert specified API url to a Kafka topic
                        println!("Message: {}", message);
                        Ok(())
                    }).map_err(|err| {
                        println!("Occurred error during the processing a message: {}", err);
                        Error::new(ErrorKind::InvalidData, err)
                    })
                })
        });

        // Run the server
        core.run(server).unwrap();
    }
}
