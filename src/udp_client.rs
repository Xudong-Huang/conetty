use std::cell::RefCell;
use std::time::Duration;
use std::io::{self, Cursor};
use std::net::ToSocketAddrs;
use std::cell::UnsafeCell;
use Client;
use errors::Error;
use may::net::UdpSocket;
use frame::{Frame, ReqBuf};

pub struct UdpClient {
    // each request would have a unique id
    id: RefCell<u64>,
    // the connection
    sock: UdpSocket,
    // send/recv buf
    buf: UnsafeCell<Vec<u8>>,
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
               buf: UnsafeCell::new(vec![0; 1024]),
           })
    }

    /// set the default timeout value
    /// the initial timeout is 1 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.sock.set_read_timeout(Some(timeout)).unwrap();
    }
}

impl Client for UdpClient {
    fn call_service(&self, req: ReqBuf) -> Result<Frame, Error> {
        let id = {
            let mut id = self.id.borrow_mut();
            *id += 1;
            *id
        };
        info!("request id = {}", id);

        // send the data to server
        self.sock.send(&(req.finish(id))).map_err(Error::from)?;

        let buf = unsafe { &mut *self.buf.get() };
        // read the response
        loop {
            self.sock.recv(buf).map_err(Error::from)?;

            // deserialize the rsp
            let rsp_frame = Frame::decode_from(&mut Cursor::new(&buf))
                .map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // disgard the rsp that is is not belong to us
            if rsp_frame.id == id {
                info!("get response id = {}", id);
                return Ok(rsp_frame);
            }
        }
    }
}
