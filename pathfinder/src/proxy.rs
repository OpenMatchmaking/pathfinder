use std::cell::{RefCell};
use std::collections::{HashMap};
use std::net::{SocketAddr};
use std::rc::{Rc};

use super::engine::{Engine};
use super::router::{Router};
use super::middleware::{Middleware, EmptyMiddleware, AuthTokenMiddleware};

use cli::{CliOptions};
use futures::sync::{mpsc};
use futures::{Future, Sink};
use futures::stream::{Stream};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::{Core};
use tokio_tungstenite::{accept_hdr_async};
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
            true => Box::new(AuthTokenMiddleware::new(cli)),
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

            let auth_middleware_callback = |req: &Request| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.path);
                println!("The request's headers are:");
                for &(ref header, ref value) in req.headers.iter() {
                    use std::str;
                    let v = str::from_utf8(value).unwrap();
                    println!("* {}: {:?}", header, v);
                }

                let auth_middleware_inner = self.auth_middleware.clone();
                let _result = auth_middleware_inner.borrow().process_request();
                Ok(None)
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
                }).or_else(|err| {
                    println!("An error occurred during the WebSocket handshake: {}", err);
                    Ok(())
                })
        });

        // Run the server
        core.run(server).unwrap();
    }
}
