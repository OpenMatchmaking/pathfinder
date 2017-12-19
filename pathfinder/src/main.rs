extern crate futures;
#[macro_use]
extern crate json;
extern crate jsonwebtoken;
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

mod auth;
mod cli;
mod config;
mod engine;
mod error;
mod proxy;

use cli::{CliOptions};
use config::{get_config};
use engine::router::{Router, extract_endpoints};
use structopt::StructOpt;
use proxy::{Proxy};


fn main() {
    let cli = CliOptions::from_args();
    let config = get_config(&cli.config);
    let endpoints = extract_endpoints(config);
    let router = Box::new(Router::new(endpoints));

    let proxy = Box::new(Proxy::new(router, &cli));
    let address = format!("{}:{}", cli.ip, cli.port).parse().unwrap();
    proxy.run(address);
}
