extern crate structopt;
#[macro_use]
extern crate structopt_derive;

mod cli;
mod config;
mod endpoint;
mod error;

use cli::{CliOptions};
use config::{get_config};
use endpoint::{extract_endpoints};
use structopt::StructOpt;


fn main() {
    let cli = CliOptions::from_args();
    let config = get_config(&cli.config);

    let endpoints = extract_endpoints(config);
    println!("{:?}", endpoints);
}
