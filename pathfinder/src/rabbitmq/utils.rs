//! Util functions for interaction with Lapin library
//

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};

use amq_protocol::uri::AMQPUri;
use log::{error, warn};

use crate::cli::CliOptions;

/// Generates a connection URL to RabbitMQ broker.
pub fn get_address_to_rabbitmq(uri: &AMQPUri) -> SocketAddr {
    let host = uri.clone().authority.host;
    let listened_port = uri.clone().authority.port;
    let address = format!("{}:{}", host, listened_port).to_socket_addrs();

    match address {
        Ok(addr) => addr.collect::<Vec<_>>()[0],
        Err(_) => {
            error!("Unable to resolve the address to the RabbitMQ \
                    node. Please, check input parameters to the RabbitMQ node \
                    or specify the certain IP-address.");
            warn!("Used the `127.0.0.1:5672` value for connection to the node.");
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5672)
        }
    }
}

/// Returns an instance of AMQPUri based on the parsed CLI options.
pub fn get_uri(cli: &CliOptions) -> AMQPUri {
    let schema = match cli.rabbitmq_secured {
        true => "amqps",
        false => "amqp",
    };
    format!(
        "{}://{}:{}@{}:{}/{}",
        schema.to_string(),
        cli.rabbitmq_username.clone(),
        cli.rabbitmq_password.clone(),
        cli.rabbitmq_host.clone(),
        cli.rabbitmq_port,
        cli.rabbitmq_virtual_host.clone()
    ).parse().unwrap_or(AMQPUri::default())
}
