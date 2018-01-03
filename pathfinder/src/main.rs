extern crate chrono;
extern crate futures;
extern crate fern;
#[macro_use]
extern crate json;
extern crate jsonwebtoken;
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


fn main() {
    let cli = CliOptions::from_args();
    match setup_logger(&cli) {
        Ok(_) => {},
        Err(err) => println!("Logger isn't instantiated: {}", err)
    };

    let config = get_config(&cli.config);
    let endpoints = extract_endpoints(config);
    let router = Box::new(Router::new(endpoints));
    let engine = Box::new(Engine::new(router));

    let proxy = Box::new(Proxy::new(engine, &cli));
    let address = format!("{}:{}", cli.ip, cli.port).parse().unwrap();
    proxy.run(address);
}
