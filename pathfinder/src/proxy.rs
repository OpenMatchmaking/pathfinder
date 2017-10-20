use std::cell::{RefCell};
use std::collections::{HashMap};
use std::net::{SocketAddr};
use std::rc::{Rc};

use super::engine::{Engine};
use super::router::{Router};

use futures::sync::{mpsc};
use futures::{Future, Sink};
use futures::stream::{Stream};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::{Core};
use tokio_tungstenite::{accept_async};
use tungstenite::protocol::{Message};


pub struct Proxy {
    engine: Rc<RefCell<Engine>>,
    connections: Rc<RefCell<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>
}


impl Proxy {
    pub fn new(router: Box<Router>) -> Proxy {
        Proxy {
            engine: Rc::new(RefCell::new(Engine::new(router))),
            connections: Rc::new(RefCell::new(HashMap::new()))
        }
    }

    pub fn run(&self, address: SocketAddr) {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let socket = TcpListener::bind(&address, &handle).unwrap();
        println!("Listening on: {}", address);

        let server = socket.incoming().for_each(|(stream, addr)| {
            let engine_inner = self.engine.clone();
            let connections_inner = self.connections.clone();
            let handle_inner = handle.clone();

            accept_async(stream)
                // Check the Auth header
                .and_then(move |ws_stream| {
                    println!("Checking the user's token");
                    Ok(ws_stream)
                })
                // Process the messages
                .and_then(move |ws_stream| {
                    // Create a channel for the stream, which other sockets will use to
                    // send us messages. It could be used for broadcasting your data to
                    // another users in the future.
                    let (tx, rx) = mpsc::unbounded();
                    connections_inner.borrow_mut().insert(addr, tx);

                    // Split the WebSocket stream so that it will be possible to work
                    // with the reading and writing halves separately.
                    let (sink, stream) = ws_stream.split();

                    // Read and process each message
                    let connections = connections_inner.clone();
                    let ws_reader = stream.for_each(move |message: Message| {
                        engine_inner.borrow().handle(&message, &addr, &connections);
                        Ok(())
                    });

                    // Write back prepared responses
                    let ws_writer = rx.fold(sink, |mut sink, msg| {
                        sink.start_send(msg).unwrap();
                        Ok(sink)
                    });

                    // Wait for either half to be done to tear down the other
                    let connection = ws_reader.map(|_| ()).map_err(|_| ())
                                              .select(ws_writer.map(|_| ()).map_err(|_| ()));

                    // Close the connection after using
                    handle_inner.spawn(connection.then(move |_| {
                        connections_inner.borrow_mut().remove(&addr);
                        println!("Connection {} closed.", addr);
                        Ok(())
                    }));

                    Ok(())
                }).or_else(|err| {
                    println!("An error occurred during the WebSocket handshake: {}", err);
                    Ok(())
                })
        });

        // Run the server
        core.run(server).unwrap();
    }
}
