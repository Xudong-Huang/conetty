use std::io;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicUsize, Ordering};
use bincode;
use io::Response;
use errors::Error;
use coroutine::net::UdpSocket;
use bincode::SizeLimit::Infinite;

pub struct UdpClient {
    // each request would have a unique id
    id: AtomicUsize,
    // default timeout is 10s
    timeout: Duration,
    // the connection
    sock: UdpSocket,
}

unsafe impl Send for UdpClient {}
unsafe impl Sync for UdpClient {}

impl UdpClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<UdpClient> {
        // this would bind a random port by the system
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.connect(addr)?;

        Ok(UdpClient {
            id: AtomicUsize::new(0),
            timeout: Duration::from_secs(10),
            sock: sock,
        })
    }

    /// set the default timeout value
    /// the initial timeout is 10 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// call the server
    /// the request must be something that is already encoded
    pub fn call_service(&self, req: &[u8]) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::with_capacity(1024);
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        info!("request id = {}", id);

        // serialize the request id
        bincode::serde::serialize_into(&mut buf, &id, Infinite)
            .map_err(|e| Error::ClientSerialize(e.to_string()))?;

        // serialize the request
        bincode::serde::serialize_into(&mut buf, &req, Infinite)
            .map_err(|e| Error::ClientSerialize(e.to_string()))?;

        // send the data to server
        self.sock.send(&buf).map_err(Error::from)?;

        // read the response
        buf.resize(1024, 0);
        self.sock.recv(&mut buf).map_err(Error::from)?;

        // deserialize the rsp
        let rsp: Response =
            bincode::serde::deserialize(&buf).map_err(|e| Error::ClientDeserialize(e.to_string()))?;

        info!("get response id = {}", rsp.id);
        assert_eq!(rsp.id, id);
        rsp.data.map_err(Error::from)
    }
}
