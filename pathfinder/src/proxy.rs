use std::cell::{RefCell};
use std::collections::{HashMap};
use std::error::{Error};
use std::net::{SocketAddr};
use std::rc::{Rc};

use engine::{Engine};
use auth::middleware::{Middleware, EmptyMiddleware};
use auth::token::middleware::{JwtTokenMiddleware};

use cli::{CliOptions};
use futures::sync::{mpsc};
use futures::{Future, Sink};
use futures::stream::{Stream};
use json::{JsonValue};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::{Core};
use tokio_tungstenite::{accept_async};
use tungstenite::protocol::{Message};


pub struct Proxy {
    engine: Rc<RefCell<Box<Engine>>>,
    connections: Rc<RefCell<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>,
    auth_middleware: Rc<RefCell<Box<Middleware>>>,
}


impl Proxy {
    pub fn new(engine: Box<Engine>, cli: &CliOptions) -> Proxy {
        let auth_middleware: Box<Middleware> = match cli.validate {
            true => Box::new(JwtTokenMiddleware::new(cli)),
               _ => Box::new(EmptyMiddleware::new(cli))
        };

        Proxy {
            engine: Rc::new(RefCell::new(engine)),
            connections: Rc::new(RefCell::new(HashMap::new())),
            auth_middleware: Rc::new(RefCell::new(auth_middleware)),
        }
    }

    pub fn run(&self, address: SocketAddr) {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let socket = TcpListener::bind(&address, &handle).unwrap();
        println!("Listening on: {}", address);

        let server = socket.incoming().for_each(|(stream, addr)| {
            let engine_local = self.engine.clone();
            let connections_local = self.connections.clone();
            let auth_middleware_local = self.auth_middleware.clone();
            let handle_local = handle.clone();

            accept_async(stream)
                // Process the messages
                .and_then(move |ws_stream| {
                    // Create a channel for the stream, which other sockets will use to
                    // send us messages. It could be used for broadcasting your data to
                    // another users in the future.
                    let (tx, rx) = mpsc::unbounded();
                    connections_local.borrow_mut().insert(addr, tx);

                    // Split the WebSocket stream so that it will be possible to work
                    // with the reading and writing halves separately.
                    let (sink, stream) = ws_stream.split();

                    // Read and process each message
                    let handle_inner = handle_local.clone();
                    let connections_inner = connections_local.clone();
                    let ws_reader = stream.for_each(move |message: Message| {

                        // Get references to required components
                        let engine_nested = engine_local.clone();
                        let connections_nested = connections_inner.clone();

                        // 1. Deserialize message into JSON
                        let json_message = match engine_local.borrow().deserialize_message(&message) {
                            Ok(json_message) => json_message,
                            Err(err) => {
                                handle_error!(&connections_inner, &addr, engine_local, err);
                                Box::new(JsonValue::new_object())
                            }
                        };

                        // 2. Apply a middleware to each incoming message
                        let auth_future = auth_middleware_local.borrow()
                            .process_request(&json_message, &handle_inner)
                            .then(move |result| {
                                if result.is_err() {
                                    handle_error!(&connections_nested, &addr, engine_nested, result.unwrap_err());
                                };
                                Ok(())
                            });

                        handle_inner.spawn(auth_future);
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
                    handle_local.spawn(connection.then(move |_| {
                        connections_local.borrow_mut().remove(&addr);
                        println!("Connection {} closed.", addr);
                        Ok(())
                    }));

                    Ok(())
                })
                // An error occurred during the WebSocket handshake
                .or_else(|err| {
                    println!("{}", err.description());
                    Ok(())
                })
        });

        // Run the server
        core.run(server).unwrap();
    }
}
