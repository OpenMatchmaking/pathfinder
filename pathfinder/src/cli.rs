//! Wrappers for interaction with CLI
//!
//! For more details about using the structopt crate you can find [here](https://github.com/TeXitoi/structopt).
//!


/// A structure that defines available arguments and options for CLI
#[derive(StructOpt, Debug)]
#[structopt(name = "Pathfinder",
            version = "0.1.0",
            about = "WebSocket-over-RabbitMQ reverse proxy")]
pub struct CliOptions {
    #[structopt(short = "c",
                long = "config",
                help = "Path to a custom settings file",
                default_value = "")]
    pub config: String,

    #[structopt(short = "i",
                long = "ip",
                help = "The used IP for a server",
                default_value = "127.0.0.1")]
    pub ip: String,

    #[structopt(short = "p",
                long = "port",
                help = "The listened port",
                default_value = "8080")]
    pub port: u16,

    #[structopt(short = "C",
                long = "cert",
                help = "Path to a SSL certificate",
                default_value = "")]
    pub ssl_certificate: String,

    #[structopt(short = "K",
                long = "key",
                help = "Path to a SSL public key",
                default_value = "")]
    pub ssl_public_key: String,

    #[structopt(short = "x",
                long = "rabbitmq-ip",
                help = "The used IP by RabbitMQ broker",
                default_value = "127.0.0.1")]
    pub rabbitmq_ip: String,

    #[structopt(short = "z",
                long = "rabbitmq-port",
                help = "The listened port by RabbitMQ broker",
                default_value = "5672")]
    pub rabbitmq_port: u16,

    #[structopt(short = "a",
                long = "rabbitmq-user",
                help = "A RabbitMQ application username",
                default_value = "user")]
    pub rabbitmq_username: String,

    #[structopt(short = "s",
                long = "rabbitmq-password",
                help = "A RabbitMQ application password",
                default_value = "password")]
    pub rabbitmq_password: String,

    #[structopt(short = "r",
                long = "redis-ip",
                help = "The used IP by Redis",
                default_value = "127.0.0.1")]
    pub redis_ip: String,

    #[structopt(short = "t",
                long = "redis-port",
                help = "The listened port by Redis",
                default_value = "6379")]
    pub redis_port: u16,

    #[structopt(short = "y",
                long = "redis-password",
                help = "Password for connecting to redis",
                default_value = "")]
    pub redis_password: String,

    #[structopt(short = "k",
                long = "jwt-secret",
                help = "Secret key for a JWT validation",
                default_value = "secret")]
    pub jwt_secret_key: String,

    #[structopt(short = "v",
                long = "validate",
                help = "Validate a token that was specified with data")]
    pub validate: bool,

    #[structopt(short = "l",
                long = "--log-level",
                help = "Verbosity level filter of the logger",
                default_value = "info")]
    pub log_level: String,
}
