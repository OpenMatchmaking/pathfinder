use std::borrow::{Cow};
use std::cell::{RefCell};
use std::collections::{HashMap};
use std::error::{Error};
use std::net::{SocketAddr};
use std::rc::{Rc};

use super::engine::{Engine};
use super::router::{Router};
use super::middleware::{Middleware, EmptyMiddleware};
use super::token::middleware::{JwtTokenMiddleware};

use cli::{CliOptions};
use futures::sync::{mpsc};
use futures::{Future, Sink};
use futures::stream::{Stream};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::{Core};
use tokio_tungstenite::{accept_hdr_async};
use tungstenite::error::{Error as TungsteniteError};
use tungstenite::handshake::server::{Request};
use tungstenite::protocol::{Message};


pub struct Proxy {
    engine: Rc<RefCell<Engine>>,
    connections: Rc<RefCell<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>,
    auth_middleware: Rc<RefCell<Box<Middleware>>>,
}


impl Proxy {
    pub fn new(router: Box<Router>, cli: &CliOptions) -> Proxy {
        let auth_middleware: Box<Middleware> = match cli.validate {
            true => Box::new(JwtTokenMiddleware::new(cli)),
               _ => Box::new(EmptyMiddleware::new(cli))
        };

        Proxy {
            engine: Rc::new(RefCell::new(Engine::new(router))),
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
            let engine_inner = self.engine.clone();
            let connections_inner = self.connections.clone();
            let handle_inner = handle.clone();

            let auth_middleware_callback = |request: &Request| {
                let auth_middleware_inner = self.auth_middleware.clone();
                let processing_result = auth_middleware_inner.borrow().process_request(request);

                match processing_result {
                    Ok(headers) => Ok(headers),
                    Err(err) => {
                        let formatted_error = format!("{}", err);
                        let message = Cow::from(formatted_error);
                        Err(TungsteniteError::Protocol(message))
                    }
                }
            };

            accept_hdr_async(stream, auth_middleware_callback)
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
