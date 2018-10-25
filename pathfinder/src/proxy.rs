//! A reverse proxy application.
//!
//! This module provides a reverse proxy implementation for the Open
//! Matchmaking project.
//!
//! The purposes of the application  are handling client connections, applying
//! a middleware with checks for a token when it was specified and communicating
//! with a message broker for getting responses from microservices in the certain
//! format.
//!

use std::collections::{HashMap};
use std::error::{Error};
use std::net::{SocketAddr};
use std::sync::{Arc, Mutex, RwLock};

use cli::{CliOptions};
use futures::sync::{mpsc};
use futures::{Future, Sink};
use futures::stream::{Stream};
use tokio::net::{TcpListener};
use tokio::reactor::{Handle};
use tokio::runtime::{run};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_current_thread::{spawn};
use tokio_tungstenite::{accept_async};
use tungstenite::protocol::{Message};

use auth::middleware::{Middleware, EmptyMiddleware};
use auth::token::middleware::{JwtTokenMiddleware};
use engine::{Engine};
use error::{PathfinderError};
use rabbitmq::{RabbitMQClient};


/// A reverse proxy application.
pub struct Proxy<'a, T: 'a> {
    engine: Arc<RwLock<Box<Engine<'a, T>>>>,
    rabbitmq_client: Arc<RwLock<Box<RabbitMQClient<'a, T>>>>,
    connections: Arc<Mutex<HashMap<SocketAddr, mpsc::UnboundedSender<Message>>>>,
    auth_middleware: Arc<RwLock<Box<Middleware>>>,
}


impl<'a, T: AsyncRead + AsyncWrite + Send + Sync + 'static> Proxy<'a, T> {
    /// Returns a new instance of a reverse proxy application.
    pub fn new(cli: &CliOptions, engine: Box<Engine<'a, T>>) -> Proxy<'a, T> {
        let auth_middleware: Box<Middleware> = match cli.validate {
            true => Box::new(JwtTokenMiddleware::new(cli)),
            _ => Box::new(EmptyMiddleware::new(cli))
        };
        let rabbitmq_client = RabbitMQClient::new(cli);

        Proxy {
            engine: Arc::new(RwLock::new(engine)),
            rabbitmq_client: Arc::new(Box::new(rabbitmq_client)),
            connections: Arc::new(Mutex::new(HashMap::new())),
            auth_middleware: Arc::new(RwLock::new(auth_middleware)),
        }
    }

    fn init(&mut self) -> impl Future<Item=(), Error=PathfinderError> {
        self.rabbitmq_client.clone().write().unwrap().init();
        Ok(())
    }

    /// Run the server on the specified address and port.
    pub fn run(&self, address: SocketAddr) {
        let handle = Handle::default();
        let listener = TcpListener::bind(&address).unwrap();
        println!("Listening on: {}", address);

        let server = listener.incoming().for_each(move |stream| {
            let addr = stream.peer_addr().expect("Connected stream should have a peer address.");

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
                    connections_local.lock().unwrap().insert(addr, tx);

                    // Split the WebSocket stream so that it will be possible to work
                    // with the reading and writing halves separately.
                    let (sink, stream) = ws_stream.split();

                    // Read and process each message
                    let handle_inner = handle_local.clone();
                    let connections_inner = connections_local.clone();
                    let ws_reader = stream.for_each(move |message: Message| {

                        // Get references to required components
                        let addr_nested = addr.clone();
                        let engine_nested = engine_local.clone();
                        let connections_nested = connections_inner.clone();
                        let transmitter_nested = &connections_nested.clone().lock().unwrap()[&addr_nested];
                        let transmitter_nested2 = transmitter_nested.clone();

                        // 1. Deserialize message into JSON
                        let json_message = match engine_local.read().unwrap().deserialize_message(&message) {
                            Ok(json_message) => json_message,
                            Err(err) => {
                                let formatted_error = format!("{}", err);
                                let error_message = engine_nested.read().unwrap().wrap_an_error(formatted_error.as_str());
                                transmitter_nested.unbounded_send(error_message).unwrap();
                                return Ok(())
                            }
                        };

                        // 2. Apply a middleware to each incoming message
                        let auth_future = auth_middleware_local.read().unwrap()
                            .process_request(json_message.clone());

                        // 3. Put request into a queue in RabbitMQ and receive the response
                        let rabbitmq_future = engine_local.read().unwrap().handle(
                            json_message.clone(), transmitter_nested.clone(), &handle_inner
                        );

                        let processing_request_future = auth_future
                            .and_then(move |_| rabbitmq_future)
                            .map_err(move |err| {
                                let formatted_error = format!("{}", err);
                                let error_message = engine_nested.read().unwrap().wrap_an_error(formatted_error.as_str());
                                transmitter_nested2.unbounded_send(error_message).unwrap();
                                ()
                            });

                        spawn(processing_request_future);
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
                    spawn(connection.then(move |_| {
                        connections_inner.lock().unwrap().remove(&addr);
                        debug!("Connection {} closed.", addr);
                        Ok(())
                    }));

                    Ok(())
                })
                // An error occurred during the WebSocket handshake
                .or_else(|err| {
                    debug!("{}", err.description());
                    Ok(())
                })
        });

        // Run the server
        run(self.init.and_then(|_| server.map_err(|_e| ())));
    }
}
