use std::{fmt, io};
use std::error::Error as StdError;

/// All errors that can occur during the use of tarpc.
#[derive(Debug)]
pub enum Error {
    /// Any IO error.
    Io(io::Error),
    /// Error in deserializing a server response.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize` or
    /// `serde::Deserialize`.
    ClientDeserialize(String),
    /// Error in serializing a client request.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize`.
    ClientSerialize(String),
    /// Error in deserializing a client request.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize` or
    /// `serde::Deserialize`.
    ServerDeserialize(String),
    /// Error in serializing a server response.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize`.
    ServerSerialize(String),
    /// The server was unable to reply to the rpc client with in some time.
    ///
    /// You can set the default timeout value in the client instance
    Timeout,
    /// The server returns an status error due to different reasons.
    ///
    /// Typically this indicates that the server is not healthy
    Status(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ClientDeserialize(ref e) => write!(f, r#"{}: "{}""#, self.description(), e),
            Error::ClientSerialize(ref e) => write!(f, r#"{}: "{}""#, self.description(), e),
            Error::ServerDeserialize(ref e) => write!(f, r#"{}: "{}""#, self.description(), e),
            Error::ServerSerialize(ref e) => write!(f, r#"{}: "{}""#, self.description(), e),
            Error::Status(ref e) => write!(f, r#"{}: "{}""#, self.description(), e),
            Error::Timeout => write!(f, r#"{}"#, self.description()),
            Error::Io(ref e) => fmt::Display::fmt(e, f),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ClientDeserialize(_) => "The client failed to deserialize the server response.",
            Error::ClientSerialize(_) => "The client failed to serialize the request.",
            Error::ServerDeserialize(_) => "The server failed to deserialize the request.",
            Error::ServerSerialize(_) => "The server failed to serialize the response.",
            Error::Status(_) => "The server returns an error code.",
            Error::Timeout => "The client get the server reply response timeout.",
            Error::Io(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::ClientDeserialize(_)
            | Error::ClientSerialize(_)
            | Error::ServerDeserialize(_)
            | Error::ServerSerialize(_)
            | Error::Status(_)
            | Error::Timeout => None,
            Error::Io(ref e) => e.cause(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<WireError> for Error {
    fn from(err: WireError) -> Self {
        match err {
            WireError::ServerDeserialize(s) => Error::ServerDeserialize(s),
            WireError::ServerSerialize(s) => Error::ServerSerialize(s),
            WireError::Status(s) => Error::Status(s),
            _ => unreachable!("unkonw WireError type received"),
        }
    }
}

/// A serializable, server-supplied error.
#[doc(hidden)]
#[derive(Debug)]
pub enum WireError {
    /// Error in deserializing a client request.
    ServerDeserialize(String),
    /// Error in serializing server response.
    ServerSerialize(String),
    /// Server Status
    Status(String),
    /// Server polling
    /// this is a special error code that used for server polling request from client
    /// client will first check this code in the very beginning before return to client rpc call
    Polling,
}
