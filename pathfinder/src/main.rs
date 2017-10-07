extern crate structopt;
#[macro_use]
extern crate structopt_derive;


mod cli;
mod config;
mod error;


use cli::{CliOptions};
use config::{get_config};
use structopt::StructOpt;


fn main() {
    let cli = CliOptions::from_args();
    let config = get_config(&cli.config);
    println!("{:?}", config);
}
