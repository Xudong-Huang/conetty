// #![deny(missing_docs)]
#[macro_use]
extern crate log;

#[doc(hidden)]
pub use errors::{Error, WireError};
#[doc(hidden)]
pub use frame::{Frame, ReqBuf, RspBuf};
#[doc(hidden)]
pub use multiplex_client::MultiplexClient;
#[doc(hidden)]
pub use server::{TcpServer, UdpServer};
#[doc(hidden)]
pub use tcp_client::TcpClient;
#[doc(hidden)]
pub use udp_client::UdpClient;

macro_rules! t {
    ($e: expr) => {
        match $e {
            Ok(val) => val,
            Err(err) => {
                error!("call = {:?}\nerr = {:?}", stringify!($e), err);
                continue;
            }
        }
    };
}

/// rpc client trait
pub trait Client {
    /// call the server
    /// the request must be encoded into the ReqBuf
    /// the response is the raw frame, you should parsing it into final response
    fn call_service(&self, req: ReqBuf) -> Result<Frame, Error>;
}

/// must impl this trait for your server
pub trait Server: Send + Sync + Sized + 'static {
    /// the service that would run in a coroutine
    /// the real request should be deserialized from the input
    /// the real response should be serialized into the RspBuf
    /// if deserialize/serialize error happened, return an Err(WireError)
    /// application error should be encap in the RspBuf
    /// here passed in a self ref to impl stateful service if you want
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError>;
}

/// Provides a few different error types.
mod errors;
/// raw frame protocol
mod frame;
mod multiplex_client;
/// Provides server framework.
mod server;
mod tcp_client;
/// Provides client impl.
mod udp_client;
