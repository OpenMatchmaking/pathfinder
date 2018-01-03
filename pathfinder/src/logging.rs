extern crate fern;
extern crate chrono;
extern crate log;

use std;

use cli::{CliOptions};

use self::log::{LevelFilter};
use self::fern::colors::{ColoredLevelConfig};


pub fn setup_logger(cli: &CliOptions) -> Result<(), fern::InitError> {
    let logging_level = match cli.log_level.parse::<LevelFilter>() {
        Ok(level) => level,
        Err(_) => {
            println!("Logging level with value={} is invalid. INFO level was set by default instead.", cli.log_level);
            println!("Use one of available logging levels: {:?}", vec![
                LevelFilter::Off, LevelFilter::Error, LevelFilter::Warn,
                LevelFilter::Info, LevelFilter::Debug, LevelFilter::Trace
            ]);
            LevelFilter::Info
        },
    };

    let colors = ColoredLevelConfig::new();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(logging_level)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
