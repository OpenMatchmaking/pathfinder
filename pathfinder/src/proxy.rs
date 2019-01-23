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

use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use amq_protocol::uri::AMQPUri;
use futures::stream::Stream;
use futures::sync::mpsc;
use futures::{Future, Sink};
use lapin_futures::error::{Error as LapinError};
use log::{debug, info, error};
use strum::AsStaticRef;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tungstenite::protocol::Message;

use crate::cli::CliOptions;
use crate::engine::{Engine, MessageSender, serialize_message, wrap_a_string_error};
use crate::error::PathfinderError;
use crate::rabbitmq::client::{RabbitMQContext, RabbitMQClient};
use crate::rabbitmq::utils::get_uri;

/// A reverse proxy application.
pub struct Proxy {
    engine: Arc<Engine>,
    amqp_uri: Arc<AMQPUri>,
    connections: Arc<Mutex<HashMap<SocketAddr, MessageSender>>>
}

impl Proxy {
    /// Returns a new instance of a reverse proxy application.
    pub fn new(cli: &CliOptions) -> Proxy {
        let engine = Engine::new(cli);
        let amqp_uri = get_uri(cli);

        Proxy {
            engine: Arc::new(engine),
            amqp_uri: Arc::new(amqp_uri),
            connections: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    /// Run the server on the specified address and the port.
    pub fn run(&self, address: SocketAddr) {
        let listener = TcpListener::bind(&address).unwrap();
        info!("Listening on: {}", address);

        let engine = self.engine.clone();
        let connections = self.connections.clone();

        let server = |rabbitmq: Arc<RabbitMQClient>| {
            listener.incoming().for_each(move |stream| {
                let addr = stream
                    .peer_addr()
                    .expect("Connected stream should have a peer address.");

                let engine_local = engine.clone();
                let rabbimq_local = rabbitmq.clone();
                let connections_local = connections.clone();

                accept_async(stream)
                    // Processing an unexpected error during creation a new connection
                    .map_err(|error| {
                        let io_error = Error::new(ErrorKind::Other, error);
                        PathfinderError::Io(io_error)
                    })
                    // Prepare lapin client context for further communication with RabbitMQ.
                    .and_then(move |ws_stream| {
                        let rabbitmq_inner = rabbimq_local.clone();
                        rabbitmq_inner
                            .get_context()
                            .map(move |rabbitmq_context: Arc<RabbitMQContext>| (ws_stream, rabbitmq_context))
                            .map_err(|error: LapinError| PathfinderError::LapinChannelError(error))
                    })
                    // Process the messages
                    .and_then(move |(ws_stream, rabbitmq_context)| {
                        let connections_inner = connections_local.clone();
                        let connection_for_insert = connections_local.clone();
                        let connection_for_remove = connections_local.clone();

                        let rabbitmq_context_inner = rabbitmq_context.clone();
                        let rabbitmq_context_for_clean = rabbitmq_context.clone();

                        // Create a channel for the stream, which other sockets will use to
                        // send us messages. It could be used for broadcasting your data to
                        // another users in the future.
                        let (tx, rx) = mpsc::unbounded();
                        connection_for_insert.lock().unwrap().insert(addr, Arc::new(tx));

                        // Split the WebSocket stream so that it will be possible to work
                        // with the reading and writing halves separately.
                        let (sink, stream) = ws_stream.split();

                        // Read and process each message
                        let ws_reader = stream.for_each(move |message: Message| {
                            // Get references to required components
                            let addr_nested = addr.clone();
                            let connections_nested = connections_inner.clone();
                            let transmitter_nested = connections_nested.lock().unwrap()[&addr_nested].clone();
                            let transmitter_for_errors = connections_nested.lock().unwrap()[&addr_nested].clone();
                            let rabbitmq_context_nested = rabbitmq_context_inner.clone();

                            let process_request_future = engine_local
                                .process_request(message, transmitter_nested, rabbitmq_context_nested)
                                .map_err(move |error: PathfinderError| {
                                    let response = match error {
                                        PathfinderError::MicroserviceError(json) => {
                                            let message = Arc::new(Box::new(json));
                                            serialize_message(message)
                                        },
                                        _ => {
                                            let error_message = format!("{}", error);
                                            let error_type = error.as_static();
                                            wrap_a_string_error(&error_type, error_message.as_str())
                                        }
                                    };

                                    transmitter_for_errors.unbounded_send(response).unwrap_or(())
                                });

                            tokio::spawn(process_request_future);
                            Ok(())
                        });

                        // Write back prepared responses
                        let ws_writer = rx.fold(sink, |mut sink, msg| {
                            sink.start_send(msg).unwrap();
                            Ok(sink)
                        });

                        // Wait for either half to be done to tear down the other
                        let connection = ws_reader
                            .map(|_| ())
                            .map_err(|_| ())
                            .select(ws_writer.map(|_| ()).map_err(|_| ()));

                        // Then clean up RabbitMQ context and close the connection after the usage
                        let handler = connection
                            .then(move |_| {
                                debug!("Clean up RabbitMQ context.");
                                rabbitmq_context_for_clean.close_channels()
                            })
                            .then(move |_| {
                                connection_for_remove.lock().unwrap().remove(&addr);
                                debug!("Connection {} closed.", addr);
                                Ok(())
                            });

                        tokio::spawn(handler);
                        Ok(())
                    })
                    // An unexpected error occurred during processing or the WebSocket handshake
                    .or_else(|error| {
                        debug!("{}", error);
                        Ok(())
                    })
            })
        };

        // Run the server
        let server_future = self
            .get_rabbitmq_client()
            .map_err(|error| error!("{}", error))
            .and_then(|rabbitmq: Arc<RabbitMQClient>| {
                server(rabbitmq)
                    .map_err(|_error| ())
            });

        tokio::runtime::run(server_future);
    }

    fn get_rabbitmq_client(&self) -> impl Future<Item=Arc<RabbitMQClient>, Error=PathfinderError> + Sync + Send + 'static {
        let amqp_uri = self.amqp_uri.clone();
        RabbitMQClient::connect(amqp_uri.as_ref())
            .map(|client| Arc::new(client))
            .map_err(|error| {
                let failure_error = error.compat().into_inner();
                PathfinderError::LapinError(failure_error)
            })
    }
}
