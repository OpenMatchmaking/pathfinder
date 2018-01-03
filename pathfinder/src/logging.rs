extern crate log;
extern crate fern;
extern crate chrono;

use std;

use cli::{CliOptions};

use self::log::{LevelFilter};


pub fn setup_logger(cli: &CliOptions) -> Result<(), fern::InitError> {
    let logging_level = match cli.log_level.parse::<LevelFilter>() {
        Ok(level) => level,
        Err(_) => {
            println!("Logging level with value={} is invalid. INFO level was set by default instead.", cli.log_level);
            LevelFilter::Info
        },
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(logging_level)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
