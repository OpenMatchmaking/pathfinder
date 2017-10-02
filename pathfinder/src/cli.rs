extern crate clap;

use self::clap::{App, ArgMatches};


pub struct Cli<'a: 'b, 'b> {
    app: App<'a, 'b>
}


impl<'a: 'b, 'b> Cli<'a, 'b>{
    pub fn new() -> Cli<'a, 'b> {
        Cli {
            app: App::new("Reverse proxy")
                .version("0.1.0")
                .author("Valeryi Savich <relrin78@gmail.com>")
                .about("WebSocket-over-Kafka reverse proxy")
                .args_from_usage(
                    "-c, --config=[FILE]      'Path to a custom settings file'
                    -i, --ip=[IP]             'The used IP for a server'
                    -p, --port=[PORT]         'The listened port'
                    -C, --cert=[CERTIFICATE]  'Path to a used certificate'
                    -K, --key=[KEY]           'Path to a public key'"
                )
        }
    }

    pub fn get_value(self, key: &'a str, default: &'a str) -> &'a str {
        let matches = self.app.get_matches();
        matches.value_of(key).unwrap_or(default)
    }
}

