extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate futures;
extern crate tokio_core;
extern crate tokio_tungstenite;
extern crate tungstenite;

mod cli;
mod config;
mod endpoint;
mod error;
mod proxy;
mod router;

use cli::{CliOptions};
use config::{get_config};
use endpoint::{extract_endpoints};
use router::{Router};
use structopt::StructOpt;
use proxy::{Proxy};


fn main() {
    let cli = CliOptions::from_args();
    let config = get_config(&cli.config);
    let endpoints = extract_endpoints(config);
    let router = Box::new(Router::new(endpoints));

    let proxy = Box::new(Proxy::new(router));
    let address = format!("{}:{}", cli.ip, cli.port).parse().unwrap();
    proxy.run(address);
}
