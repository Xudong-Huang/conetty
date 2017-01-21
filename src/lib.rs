// #![deny(missing_docs)]
#[doc(hidden)]
extern crate coroutine;
#[macro_use]
extern crate log;
#[doc(hidden)]
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[doc(hidden)]
extern crate bincode;
#[doc(hidden)]
extern crate comanaged;

pub use errors::Error;

// #[doc(hidden)]
// pub use client::{TcpClient, UdpClient};
// #[doc(hidden)]
// pub use server::{TcpServer, UdpServer};
#[doc(hidden)]
pub use errors::WireError;

/// Provides client framework.
pub mod client;
/// Provides server framework.
pub mod server;
/// Provides a few different error types.
mod errors;
