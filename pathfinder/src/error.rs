use std::io;
use std::error;
use std::fmt;
use std::result;


pub type Result<T> = result::Result<T, PathfinderError>;


#[derive(Debug)]
pub enum PathfinderError {
    Io(io::Error),
}


impl fmt::Display for PathfinderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PathfinderError::Io(ref err) => write!(f, "IO error: {}", err),
        }
    }
}


impl error::Error for PathfinderError {
    fn description(&self) -> &str {
        match *self {
            PathfinderError::Io(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            PathfinderError::Io(ref err) => Some(err),
        }
    }
}


impl From<io::Error> for PathfinderError {
    fn from(err: io::Error) -> PathfinderError {
        PathfinderError::Io(err)
    }
}
