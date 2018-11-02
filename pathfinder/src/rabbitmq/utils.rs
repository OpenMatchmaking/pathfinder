//! Util functions for interaction with Lapin library
//

use std::net::SocketAddr;

use amq_protocol::uri::AMQPUri;

use super::super::cli::CliOptions;

/// Generates a connection URL to RabbitMQ broker.
pub fn get_address_to_rabbitmq(uri: &AMQPUri) -> SocketAddr {
    let host = uri.clone().authority.host;
    let listened_port = uri.clone().authority.port;
    format!("{}:{}", host, listened_port).parse().unwrap()
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
