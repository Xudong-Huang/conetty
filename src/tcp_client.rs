use std::cell::RefCell;
use std::time::Duration;
use std::io::{self, Cursor};
use std::net::ToSocketAddrs;
use Client;
use response;
use frame::Frame;
use errors::Error;
use bufstream::BufStream;
use coroutine::net::TcpStream;

pub struct TcpClient {
    // each request would have a unique id
    id: RefCell<u64>,
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

        // encode the request
        Frame::encode_into(s, id, req).map_err(Error::from)?;

        // read the response
        loop {
            // deserialize the rsp
            let rsp_frame =
                Frame::decode_from(s).map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            let rsp = response::decode_from(&mut Cursor::new(&rsp_frame.data));

            // disgard the rsp that is is not belong to us
            if rsp_frame.id == id {
                info!("get response id = {}", id);
                return rsp;
            }
        }
    }
}
