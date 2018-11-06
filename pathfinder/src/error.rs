//! Error handlers for an application
//!
//! This module is intended for simplifying error handling and propagating
//! them in an existing application, easy to use and quite extendable in
//! the future.
//!

extern crate config;

use std::error;
use std::fmt;
use std::io;
use std::result;

use self::config::ConfigError;

/// Type alias for `Result` objects that return a Pathfinder error.
pub type Result<T> = result::Result<T, PathfinderError>;

/// An enum of all possible errors which could occur during the work of reverse proxy.
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
    /// converting into JSON).
    DecodingError(String),
    /// The error that occurred when token isn't specified or invalid.
    AuthenticationError(String),
    /// The error that occurred with a message broker.
    MessageBrokerError(String),
}

impl fmt::Display for PathfinderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PathfinderError::Io(ref err) => write!(f, "IO error: {}", err),
            PathfinderError::SettingsError(ref err) => write!(f, "Settings error: {}", err),
            PathfinderError::InvalidEndpoint(ref msg) => write!(f, "Parse error: {}", msg),
            PathfinderError::EndpointNotFound(ref msg) => write!(f, "Endpoint \"{}\" was not found", msg),
            PathfinderError::DecodingError(ref msg) => write!(f, "Decoding error: {}", msg),
            PathfinderError::AuthenticationError(ref msg) => write!(f, "Authentication error: {}", msg),
            PathfinderError::MessageBrokerError(ref msg) => write!(f, "{}", msg),
        }
    }
}

impl error::Error for PathfinderError {
    fn description(&self) -> &str {
        match *self {
            PathfinderError::Io(ref err) => err.description(),
            PathfinderError::SettingsError(ref err) => err.description(),
            PathfinderError::InvalidEndpoint(ref msg) => msg,
            PathfinderError::EndpointNotFound(ref msg) => msg,
            PathfinderError::DecodingError(ref msg) => msg,
            PathfinderError::AuthenticationError(ref msg) => msg,
            PathfinderError::MessageBrokerError(ref msg) => msg
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
