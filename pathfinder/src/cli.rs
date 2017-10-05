extern crate clap;

use std::cell::RefCell;
use self::clap::{App, ArgMatches};


pub fn cli<'a>() -> ArgMatches<'a> {
    App::new("Pathfinder")
        .version("0.1.0")
        .author("Valeryi Savich <relrin78@gmail.com>")
        .about("WebSocket-over-Kafka reverse proxy")
        .args_from_usage(
           "-c, --config=[FILE]       'Path to a custom settings file'
            -i, --ip=[IP]             'The used IP for a server'
            -p, --port=[PORT]         'The listened port'
            -C, --cert=[CERTIFICATE]  'Path to a used certificate'
            -K, --key=[KEY]           'Path to a public key'",
        )
        .get_matches()
}


pub fn get_value<'a>(cli: &'a ArgMatches<'a>, key: &'a str, default: &'a str) -> &'a str {
    let cli = RefCell::new(cli);
    let cli = cli.into_inner();
    cli.value_of(key).unwrap_or(default)
}


#[cfg(test)]
mod tests {
    use super::clap::{App, Arg};
    use super::{cli, get_value};

    #[test]
    fn test_get_value_returns_extracted_value() {
        let cli = App::new("Test CLI")
                    .version("0.1.0")
                    .arg(Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .default_value("test")
                    )
                   .get_matches();
        assert_eq!(get_value(&cli, "config", "not found"), "test");
    }

    #[test]
    fn test_get_value_returns_default_value() {
        let cli = cli();
        assert_eq!(get_value(&cli, "WRONG_KEY", "not found"), "not found");
    }
}