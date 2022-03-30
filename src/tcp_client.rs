use std::io::{self, BufReader, Write};
use std::net::ToSocketAddrs;
use std::time::Duration;

use crate::errors::Error;
use crate::frame::{Frame, ReqBuf};
use crate::SimpleClient;
use may::net::TcpStream;

#[derive(Debug)]
pub struct TcpClient {
    // each request would have a unique id
    id: u64,
    // the connection
    sock: BufReader<TcpStream>,
}

impl TcpClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<TcpClient> {
        // this would bind a random port by the system
        let sock = TcpStream::connect(addr)?;
        sock.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

        Ok(TcpClient {
            id: 0,
            sock: BufReader::with_capacity(1024, sock),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 5 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.sock.get_ref().set_read_timeout(Some(timeout)).unwrap();
    }
}

impl SimpleClient for TcpClient {
    fn call_service(&mut self, req: ReqBuf) -> Result<Frame, Error> {
        let id = self.id;
        self.id += 1;
        info!("request id = {}", id);

        // encode the request
        self.sock.get_mut().write_all(&(req.finish(id)))?;

        // read the response
        loop {
            // deserialize the rsp
            let rsp_frame = Frame::decode_from(&mut self.sock)
                .map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // discard the rsp that is is not belong to us
            if rsp_frame.id == id {
                info!("get response id = {}", id);
                return Ok(rsp_frame);
            }
        }
    }
}
