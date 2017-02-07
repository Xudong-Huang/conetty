use std::io;
use std::cell::RefCell;
use std::time::Duration;
use std::net::ToSocketAddrs;
use Client;
use Response;
use frame::Frame;
use errors::Error;
use bincode::serde as encode;
use coroutine::net::UdpSocket;

pub struct UdpClient {
    // each request would have a unique id
    id: RefCell<u64>,
    // the connection
    sock: UdpSocket,
    // send/recv buf
    buf: Vec<u8>,
}

// the UdpClient is Send but not Sync
unsafe impl Send for UdpClient {}

impl UdpClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<UdpClient> {
        // this would bind a random port by the system
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.connect(addr)?;
        sock.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

        Ok(UdpClient {
            id: RefCell::new(0),
            sock: sock,
            buf: Vec::with_capacity(1024),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 1 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.sock.set_read_timeout(Some(timeout)).unwrap();
    }
}

impl Client for UdpClient {
    fn call_service(&self, req: &[u8]) -> Result<Vec<u8>, Error> {
        let id = {
            let mut id = self.id.borrow_mut();
            *id += 1;
            *id
        };
        info!("request id = {}", id);

        let me = unsafe { &mut *(self as *const _ as *mut Self) };
        let buf = &mut me.buf;

        buf.resize(0, 0);
        Frame::encode_into(buf, id, req).map_err(Error::from)?;

        // send the data to server
        self.sock.send(&buf).map_err(Error::from)?;

        // read the response
        buf.resize(1024, 0);
        loop {
            self.sock.recv(buf).map_err(Error::from)?;

            // deserialize the rsp
            let rsp: Response =
                encode::deserialize(&buf).map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // disgard the rsp that is is not belong to us
            if rsp.id == id {
                info!("get response id = {}", rsp.id);
                // return rsp.data.map_err(Error::from);
                return Ok(rsp.data);
            }
        }
    }
}
