use std::cell::RefCell;
use std::io::{self, BufReader, Write};
use std::net::ToSocketAddrs;
use std::time::Duration;

use crate::errors::Error;
use crate::frame::{Frame, ReqBuf};
use crate::Client;
use may::net::TcpStream;

#[derive(Debug)]
pub struct TcpClient {
    // each request would have a unique id
    id: RefCell<u64>,
    // the connection
    sock: BufReader<TcpStream>,
}

// the TcpClient is Send but not Sync
unsafe impl Send for TcpClient {}

impl TcpClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<TcpClient> {
        // this would bind a random port by the system
        let sock = TcpStream::connect(addr)?;
        sock.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

        Ok(TcpClient {
            id: RefCell::new(0),
            sock: BufReader::with_capacity(1024, sock),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 5 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.sock.get_ref().set_read_timeout(Some(timeout)).unwrap();
    }
}

impl Client for TcpClient {
    fn call_service(&self, req: ReqBuf) -> Result<Frame, Error> {
        let id = {
            let mut id = self.id.borrow_mut();
            *id += 1;
            *id
        };
        info!("request id = {}", id);

        #[allow(clippy::cast_ref_to_mut)]
        let me = unsafe { &mut *(self as *const _ as *mut Self) };
        let s = &mut me.sock;

        // encode the request
        s.get_mut().write_all(&(req.finish(id)))?;

        // read the response
        loop {
            // deserialize the rsp
            let rsp_frame =
                Frame::decode_from(s).map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // disgard the rsp that is is not belong to us
            if rsp_frame.id == id {
                info!("get response id = {}", id);
                return Ok(rsp_frame);
            }
        }
    }
}
