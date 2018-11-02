//! Logging module
//!
//! This module provides a logging opportunities for the pathfinder application
//! so that all output in console will be handled by fern logging tool.
//!
//! # Useful links
//! * [log crate documentation](https://docs.rs/log)
//! * [fern crate documentation](https://docs.rs/fern/*/fern/)
//!

extern crate chrono;
extern crate fern;
extern crate log;

use std;

use cli::CliOptions;

use self::fern::colors::ColoredLevelConfig;
use self::log::LevelFilter;

/// Initialize a logger from the fern crate.
pub fn setup_logger(cli: &CliOptions) -> Result<(), fern::InitError> {
    let logging_level = match cli.log_level.parse::<LevelFilter>() {
        Ok(level) => level,
        Err(_) => {
            println!(
                "Logging level with value={} is invalid. INFO level was set by default instead.",
                cli.log_level
            );
            println!(
                "Use one of available logging levels: {:?}",
                vec![
                    LevelFilter::Off,
                    LevelFilter::Error,
                    LevelFilter::Warn,
                    LevelFilter::Info,
                    LevelFilter::Debug,
                    LevelFilter::Trace
                ]
            );
            LevelFilter::Info
        }
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
        }).level(logging_level)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
