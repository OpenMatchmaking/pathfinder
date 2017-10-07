extern crate structopt;


#[derive(StructOpt, Debug)]
#[structopt(name = "Pathfinder",
            version = "0.1.0",
            author = "Valeryi Savich <relrin78@gmail.com>",
            about = "WebSocket-over-Kafka reverse proxy")]
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
    pub port: i32,

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
                long = "kafka-ip",
                help = "The used IP by Kafka broker",
                default_value = "127.0.0.1")]
    pub kafka_ip: String,

    #[structopt(short = "z",
                long = "kafka-port",
                help = "The listened port by Kafka broker",
                default_value = "8080")]
    pub kafka_port: i32,
}
