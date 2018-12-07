//! WebSocket-over-RabbitMQ reverse proxy
//!

pub mod cli;
pub mod config;
#[macro_use]
pub mod engine;
pub mod error;
pub mod logging;
pub mod proxy;
pub mod rabbitmq;

use log::warn;
use structopt::StructOpt;

use crate::cli::CliOptions;
use crate::logging::setup_logger;
use crate::proxy::Proxy;

fn main() {
    let cli = CliOptions::from_args();
    match setup_logger(&cli) {
        Ok(_) => {}
        Err(err) => warn!("Logger isn't instantiated: {}", err),
    };

    let proxy = Box::new(Proxy::new(&cli));
    let address = format!("{}:{}", cli.ip, cli.port).parse().unwrap();
    proxy.run(address);
}
