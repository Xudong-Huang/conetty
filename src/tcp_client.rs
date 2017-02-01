use std::cell::RefCell;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::io::{self, Write};
use Client;
use Response;
use errors::Error;
use bufstream::BufStream;
use bincode::serde as encode;
use coroutine::net::TcpStream;
use bincode::SizeLimit::Infinite;

pub struct TcpClient {
    // each request would have a unique id
    id: RefCell<usize>,
    // the connection
    sock: BufStream<TcpStream>,
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
            sock: BufStream::with_capacities(1024, 1024, sock),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 5 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.sock.get_ref().set_read_timeout(Some(timeout)).unwrap();
    }
}

impl Client for TcpClient {
    fn call_service(&self, req: &[u8]) -> Result<Vec<u8>, Error> {
        let id = {
            let mut id = self.id.borrow_mut();
            *id += 1;
            *id
        };
        info!("request id = {}", id);

        let me = unsafe { &mut *(self as *const _ as *mut Self) };
        let s = &mut me.sock;

        // serialize the request id
        encode::serialize_into(s, &id, Infinite)
            .map_err(|e| Error::ClientSerialize(e.to_string()))?;

        // serialize the request
        encode::serialize_into(s, &req, Infinite)
            .map_err(|e| Error::ClientSerialize(e.to_string()))?;

        // send the request
        s.flush().unwrap();

        // read the response
        loop {
            // deserialize the rsp
            let rsp: Response = encode::deserialize_from(s, Infinite)
                .map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // disgard the rsp that is is not belong to us
            if rsp.id == id {
                info!("get response id = {}", rsp.id);
                return rsp.data.map_err(Error::from);
            }
        }
    }
}
