mod cli;

use cli::{Cli};


fn main() {
    let cli = Cli::new();
    //let matches = cli.get_matches();

    //let config = matches.value_of("config").unwrap_or("default.yaml");
    let config = cli.get_value("config", "default.yaml");
    println!("Value for config: {}", config);
}
