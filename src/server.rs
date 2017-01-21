use std::io;
use std::sync::Arc;
use std::net::ToSocketAddrs;
use bincode;
use coroutine;
use errors::WireError;
// use comanaged::Manager;
use coroutine::net::UdpSocket;
use bincode::SizeLimit::Infinite;
// use serde::{Deserialize, Serialize};

/// raw request/response wrapper
#[derive(Serialize, Deserialize)]
pub struct Request {
    id: usize,
    data: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
pub struct Response {
    id: usize,
    data: Result<Vec<u8>, WireError>,
}

macro_rules! t {
    ($e: expr) => (match $e {
        Ok(val) => val,
        Err(err) => {
            error!("call = {:?}\nerr = {:?}", stringify!($e), err);
            continue;
        }
    })
}

// pub trait Codec {
//     fn decode<T: Deserialize>(v: &[u8]) -> Result<T, String>;
//     fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String>;
//     fn recv_frame<T: Read>(io: &mut T) -> Result<Frame, String>;
//     fn send_frame<T: Write>(io: &mut T, frame: &Frame) -> Result<(), String>;
// }
//
// pub struct BinCodeCodec;
//
// impl Codec for BinCodeCodec {
//     fn decode<T: Deserialize>(v: &[u8]) -> Result<T, String> {
//         bincode::serde::deserialize(v).map_err(|e| e.to_string())
//     }
//     fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
//         bincode::serde::serialize(value, Infinite).map_err(|e| e.to_string())
//     }
//
//     fn recv_frame<T: Read>(io: &mut T) -> Result<Frame, String> {
//         bincode::serde::deserialize_from(io, Infinite).map_err(|e| e.to_string())
//     }
//
//     fn send_frame<T: Write>(io: &mut T, frame: &Frame) -> Result<(), String> {
//         bincode::serde::serialize_into(io, frame, Infinite).map_err(|e| e.to_string())
//     }
// }


/// must impl this trait for your server
pub trait Service: Send + Sync + Sized + 'static {
    /// the service that would run in a coroutine
    /// the real request should be deserialized from the input
    /// the real response shoudl be serialized into the raw vec
    /// if deserialize/serialize error happened, return an Err(WireError)
    /// application error should be encap in the raw vec
    /// here passed in a self ref to impl stateful service if you want
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError>;
}

/// Provides a function for starting the service.
pub trait UdpService: Service {
    /// Spawns the service, binding to the given address
    /// return a coroutine that you can cancel it when need to stop the service
    fn start<L: ToSocketAddrs>(self, addr: L) -> io::Result<coroutine::JoinHandle<()>> {
        // fn call_service(buf: &[u8], addr: SocketAddr) -> Result<Frame, WireError> {}
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
                    match sock.send_to(&data, addr) {
                        Ok(_) => {}
                        Err(err) => return error!("udp send_to failed, err={:?}", err),
                    }

                });
            }
        })
    }
}

impl<T: Service> UdpService for T {}
