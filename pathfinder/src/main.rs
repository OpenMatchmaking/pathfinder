//! WebSocket-over-RabbitMQ reverse proxy
//!

extern crate amq_protocol;
extern crate chrono;
extern crate clap;
extern crate futures;
extern crate fern;
#[macro_use]
extern crate json;
extern crate lapin_futures_rustls;
extern crate lapin_futures_tls_api;
extern crate lapin_futures_tls_internal;
#[macro_use]
extern crate log;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_tcp;
extern crate tokio_tungstenite;
extern crate tungstenite;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate uuid;

pub mod cli;
pub mod config;
#[macro_use]
pub mod engine;
pub mod error;
pub mod logging;
pub mod proxy;
pub mod rabbitmq;

use cli::{CliOptions};
use structopt::StructOpt;
use logging::{setup_logger};
use proxy::{Proxy};


fn main() {
    let cli = CliOptions::from_args();
    match setup_logger(&cli) {
        Ok(_) => {},
        Err(err) => println!("Logger isn't instantiated: {}", err)
    };

    let proxy = Box::new(Proxy::new(&cli));
    let address = format!("{}:{}", cli.ip, cli.port).parse().unwrap();
    proxy.run(address);
}
