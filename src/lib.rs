// #![deny(missing_docs)]
#[doc(hidden)]
pub extern crate coroutine;
#[macro_use]
extern crate log;
#[doc(hidden)]
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[doc(hidden)]
pub extern crate bincode;
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
pub use server::{UdpServer, TcpServer};
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

/// raw request/response wrapper
#[derive(Serialize, Deserialize)]
struct Request {
    pub id: usize,
    pub data: Vec<u8>,
}

/// raw request/response wrapper
#[derive(Serialize, Deserialize)]
struct Response {
    pub id: usize,
    pub data: Result<Vec<u8>, WireError>,
}

/// rpc client trait
pub trait Client {
    /// call the server
    /// the request must be something that is already encoded
    /// the response is serialized into Vec<u8>
    fn call_service(&self, req: &[u8]) -> Result<Vec<u8>, Error>;
}

/// must impl this trait for your server
pub trait Server: Send + Sync + Sized + 'static {
    /// the service that would run in a coroutine
    /// the real request should be deserialized from the input
    /// the real response should be serialized into the raw vec
    /// if deserialize/serialize error happened, return an Err(WireError)
    /// application error should be encap in the raw vec
    /// here passed in a self ref to impl stateful service if you want
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError>;
}


/// Provides client impl.
mod udp_client;
mod tcp_client;
mod multiplex_client;
/// Provides server framework.
mod server;
/// Provides a few different error types.
mod errors;
