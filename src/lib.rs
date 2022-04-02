#[macro_use]
extern crate log;

pub use errors::{Error, WireError};
pub use frame::{Frame, ReqBuf, RspBuf};
pub use multiplex_client::{MultiplexClient, TryClone};
pub use server::{TcpServer, UdpServer};
pub use stream_client::{SetTimeout, StreamClient};
pub use udp_client::UdpClient;

#[cfg(unix)]
pub use server::UdsServer;

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
    /// application error should be encapsulated into the RspBuf
    /// here passed in a self ref to impl stateful service if you want
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError>;
}

/// Provides a few different error types
mod errors;
/// raw frame protocol
mod frame;
mod multiplex_client;
mod queued_writer;
/// Provides server framework
mod server;

/// Provide stream client
mod stream_client;
/// Provides udp client
mod udp_client;
