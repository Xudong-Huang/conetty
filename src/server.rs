use std::io;
use std::sync::Arc;
use std::net::ToSocketAddrs;
use bincode;
use coroutine;
use errors::WireError;
// use comanaged::Manager;
use io::{Request, Response};
use coroutine::net::UdpSocket;
use bincode::SizeLimit::Infinite;

macro_rules! t {
    ($e: expr) => (match $e {
        Ok(val) => val,
        Err(err) => {
            error!("call = {:?}\nerr = {:?}", stringify!($e), err);
            continue;
        }
    })
}

/// must impl this trait for your server
pub trait Service: Send + Sync + Sized + 'static {
    /// the service that would run in a coroutine
    /// the real request should be deserialized from the input
    /// the real response should be serialized into the raw vec
    /// if deserialize/serialize error happened, return an Err(WireError)
    /// application error should be encap in the raw vec
    /// here passed in a self ref to impl stateful service if you want
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError>;
}

/// Provides a function for starting the service.
pub trait UdpServer: Service {
    /// Spawns the service, binding to the given address
    /// return a coroutine that you can cancel it when need to stop the service
    fn start<L: ToSocketAddrs>(self, addr: L) -> io::Result<coroutine::JoinHandle<()>> {
        let sock = Arc::new(UdpSocket::bind(addr)?);
        let server = Arc::new(self);
        coroutine::Builder::new().name("UdpServer".to_owned()).spawn(move || {
            let mut buf = vec![0u8; 1024];
            loop {
                let server = server.clone();
                let sock = sock.clone();
                // each udp packet should be less than 1024 bytes
                let (len, addr) = t!(sock.recv_from(&mut buf));
                info!("recv_from: len={:?} addr={:?}", len, addr);

                // if we failed to deserialize the request frame, just continue
                let req: Request = t!(bincode::serde::deserialize(&buf));

                coroutine::spawn(move || {
                    let rsp = Response {
                        id: req.id,
                        data: server.service(&req.data),
                    };

                    // serialize the result, ignore the error
                    let data = match bincode::serde::serialize(&rsp, Infinite) {
                        Ok(data) => data,
                        Err(err) => return error!("udp server serialize failed, err={:?}", err),
                    };

                    info!("send_to: len={:?} addr={:?}", data.len(), addr);

                    // send the result back to client
                    // udp no need to proect by a mutex, each send would be one frame
                    match sock.send_to(&data, addr) {
                        Ok(_) => {}
                        Err(err) => return error!("udp send_to failed, err={:?}", err),
                    }
                });
            }
        })
    }
}

impl<T: Service> UdpServer for T {}
