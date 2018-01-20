//! WebSocket-over-RabbitMQ reverse proxy
//!

extern crate chrono;
extern crate clap;
extern crate futures;
extern crate fern;
#[macro_use]
extern crate json;
extern crate jsonwebtoken;
extern crate lapin_futures_rustls;
extern crate lapin_futures_tls_api;
#[macro_use]
extern crate log;
extern crate tokio_core;
extern crate tokio_tungstenite;
extern crate tungstenite;
#[macro_use]
extern crate redis_async;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate uuid;

pub mod auth;
pub mod cli;
pub mod config;
#[macro_use]
pub mod engine;
pub mod error;
pub mod logging;
pub mod proxy;

use cli::{CliOptions};
use config::{get_config};
use engine::{Engine};
use engine::router::{Router, extract_endpoints};
use structopt::StructOpt;
use logging::{setup_logger};
use proxy::{Proxy};

use lapin_futures_rustls::lapin;
use futures::future::Future;
use lapin::channel::ConfirmSelectOptions;
use tokio_core::reactor::Core;
use engine::rabbitmq::client::{RabbitMQClient};



fn main() {
    let cli = CliOptions::from_args();
//    match setup_logger(&cli) {
//        Ok(_) => {},
//        Err(err) => println!("Logger isn't instantiated: {}", err)
//    };
//
//    let config = get_config(&cli.config);
//    let endpoints = extract_endpoints(config);
//    let router = Box::new(Router::new(endpoints));
//    let engine = Box::new(Engine::new(&cli, router));
//
//    let proxy = Box::new(Proxy::new(&cli, engine));
//    let address = format!("{}:{}", cli.ip, cli.port).parse().unwrap();
//    proxy.run(address);

    let mut core = Core::new().unwrap();
    let handle   = core.handle();

    let client = RabbitMQClient::new(&cli);
    let new_future = client.get_future(&handle);

    core.run(
        new_future
            .and_then(|client| {
                println!("Connected!");
                client.create_confirm_channel(ConfirmSelectOptions::default())
            }).and_then(|channel| {
                println!("Closing channel.");
                channel.close(200, "Bye")
            })
    ).unwrap();
}
