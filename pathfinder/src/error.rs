extern crate config;

use std::io;
use std::error;
use std::fmt;
use std::result;

use self::config::{ConfigError};


/// Helper alias for `Result` objects that return a Pathfinder error.
pub type Result<T> = result::Result<T, PathfinderError>;


#[derive(Debug)]
pub enum PathfinderError {
    /// The error that occurred during work with I/O.
    Io(io::Error),
    /// Represents all possible errors that can occur when working with
    /// configuration (reading, watching for a changes, etc.).
    SettingsError(ConfigError),
    /// Occurs when an inner structure of an endpoint is invalid.
    /// For example: missed fields or invalid format.
    InvalidEndpoint(String),
    /// Occurs when router is trying to get an access to an endpoint, that
    /// doesn't not exist.
    EndpointNotFound(String),
    /// Occurs during processing an incoming message (e.g. parsing,
    /// converting into JSON)
    DecodingError(String),
}


impl fmt::Display for PathfinderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PathfinderError::Io(ref err) => write!(f, "IO error: {}", err),
            PathfinderError::SettingsError(ref err) => write!(f, "Settings error: {}", err),
            PathfinderError::InvalidEndpoint(ref s) => write!(f, "Parse error: {}", s),
            PathfinderError::EndpointNotFound(ref s) => write!(f, "Endpoint \"{}\" was not found", s),
            PathfinderError::DecodingError(ref s) => write!(f, "Decoding error: {} ", s),
        }
    }
}


impl error::Error for PathfinderError {
    fn description(&self) -> &str {
        match *self {
            PathfinderError::Io(ref err) => err.description(),
            PathfinderError::SettingsError(ref err) => err.description(),
            _ => "configuration error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            PathfinderError::Io(ref err) => Some(err),
            PathfinderError::SettingsError(ref err) => Some(err),
            _ => None,
        }
    }
}


impl From<io::Error> for PathfinderError {
    fn from(err: io::Error) -> PathfinderError {
        PathfinderError::Io(err)
    }
}


impl From<config::ConfigError> for PathfinderError {
    fn from(err: config::ConfigError) -> PathfinderError {
        PathfinderError::SettingsError(err)
    }
}
