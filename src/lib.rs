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
#[doc(hidden)]
extern crate bufstream;

pub use errors::Error;

#[doc(hidden)]
pub use udp_client::UdpClient;
#[doc(hidden)]
pub use tcp_client::TcpClient;
#[doc(hidden)]
pub use multiplex_client::MultiPlexClient;
#[doc(hidden)]
pub use server::{Service, UdpServer, TcpServer};
#[doc(hidden)]
pub use errors::WireError;

macro_rules! t {
    ($e: expr) => (match $e {
        Ok(val) => val,
        Err(err) => {
            error!("call = {:?}\nerr = {:?}", stringify!($e), err);
            continue;
        }
    })
}

/// Provides client impl.
mod udp_client;
mod tcp_client;
mod multiplex_client;
/// Provides server framework.
mod server;
/// Provides a few different error types.
mod errors;
/// Provides request/response definition
mod io;
