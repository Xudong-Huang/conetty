use std::io;
use std::sync::Arc;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::marker::PhantomData;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use bincode;
use coroutine;
use io::Response;
use errors::Error;
use coroutine::net::UdpSocket;
use bincode::SizeLimit::Infinite;
use coroutine::sync::{Mutex, SyncBlocker};

struct WaitReq {
    blocker: Arc<SyncBlocker>,
    rsp: Option<Result<Vec<u8>, Error>>,
}

struct WaitReqMap {
    map: Mutex<HashMap<usize, *mut WaitReq>>,
}

unsafe impl Send for WaitReqMap {}
unsafe impl Sync for WaitReqMap {}

impl WaitReqMap {
    pub fn new() -> Self {
        WaitReqMap { map: Mutex::new(HashMap::new()) }
    }

    pub fn add(&self, id: usize, req: &mut WaitReq) {
        let mut m = self.map.lock().unwrap();
        m.insert(id, req as *mut _);
    }

    pub fn get(&self, id: usize) -> Option<&mut WaitReq> {
        let mut m = self.map.lock().unwrap();
        m.remove(&id).map(|v| { unsafe { &mut *v } })
    }
}

// wait for response
fn wait_rsp(req_map: &WaitReqMap, id: usize, timeout: Duration) -> Result<Vec<u8>, Error> {
    let cur = SyncBlocker::current();
    let mut req = WaitReq {
        blocker: cur.clone(),
        rsp: None,
    };
    req_map.add(id, &mut req);

    use coroutine::ParkError;
    match cur.park(Some(timeout)) {
        Ok(_) => {
            match req.rsp.take() {
                Some(d) => d,
                None => {
                    println!("can't get rsp, id={}", id);
                    panic!("unable to get the rsp");
                }
            }
        }
        Err(ParkError::Timeout) => {
            // remove the req from req map
            println!("timeout zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz, id={}", id);
            req_map.get(id);
            Err(Error::Timeout)
        }
        Err(ParkError::Canceled) => {
            req_map.get(id);
            coroutine::trigger_cancel_panic();
        }
    }
}

pub struct UdpClient {
    // each request would have a unique id
    id: AtomicUsize,
    // the connection
    sock: UdpSocket,
    // disable Sync
    _mark: PhantomData<*mut usize>,
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
            id: AtomicUsize::new(0),
            sock: sock,
            _mark: PhantomData,
        })
    }

    /// set the default timeout value
    /// the initial timeout is 1 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.sock.set_read_timeout(Some(timeout)).unwrap();
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
        loop {
            self.sock.recv(&mut buf).map_err(Error::from)?;

            // deserialize the rsp
            let rsp: Response =
                bincode::serde::deserialize(&buf).map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // disgard the rsp that is is not belong to us
            if rsp.id == id {
                info!("get response id = {}", rsp.id);
                return rsp.data.map_err(Error::from);
            }
        }
    }
}
