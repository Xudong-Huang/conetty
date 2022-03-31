use std::io::{self, BufReader, Read, Write};
use std::time::Duration;

use crate::errors::Error;
use crate::frame::{Frame, ReqBuf};

pub trait SetTimeout {
    fn set_timeout(&mut self, timeout: Duration) -> Result<(), io::Error>;
}

impl SetTimeout for std::net::TcpStream {
    fn set_timeout(&mut self, timeout: Duration) -> Result<(), io::Error> {
        self.set_read_timeout(Some(timeout))
    }
}
impl SetTimeout for may::net::TcpStream {
    fn set_timeout(&mut self, timeout: Duration) -> Result<(), io::Error> {
        self.set_read_timeout(Some(timeout))
    }
}
#[cfg(unix)]
impl SetTimeout for std::os::unix::net::UnixStream {
    fn set_timeout(&mut self, timeout: Duration) -> Result<(), io::Error> {
        self.set_read_timeout(Some(timeout))
    }
}
#[cfg(unix)]
impl SetTimeout for may::os::unix::net::UnixStream {
    fn set_timeout(&mut self, timeout: Duration) -> Result<(), io::Error> {
        self.set_read_timeout(Some(timeout))
    }
}

pub struct StreamClient<W: Write + Read> {
    // each request would have a unique id
    id: u64,
    // the connection
    stream: BufReader<W>,
}

impl<W: Write + Read> StreamClient<W> {
    /// connect to the server address
    pub fn new(stream: W) -> Self {
        StreamClient {
            id: 0,
            stream: BufReader::with_capacity(1024, stream),
        }
    }
}

impl<W: Write + Read + SetTimeout> StreamClient<W> {
    /// set timeout
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), io::Error> {
        self.stream.get_mut().set_timeout(timeout)
    }
}

impl<W: Write + Read> StreamClient<W> {
    /// call the server
    /// the request must be encoded into the ReqBuf
    /// the response is the raw frame, you should parsing it into final response
    pub fn call_service(&mut self, req: ReqBuf) -> Result<Frame, Error> {
        let id = self.id;
        self.id += 1;
        info!("request id = {}", id);

        // encode the request
        self.stream.get_mut().write_all(&(req.finish(id)))?;

        // read the response
        loop {
            // deserialize the rsp
            let rsp_frame = Frame::decode_from(&mut self.stream)
                .map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // discard the rsp that is is not belong to us
            if rsp_frame.id == id {
                info!("get response id = {}", id);
                return Ok(rsp_frame);
            }
        }
    }
}
