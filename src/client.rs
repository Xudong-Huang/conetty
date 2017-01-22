use std::io;
use std::sync::Arc;
use std::time::Duration;
use std::net::ToSocketAddrs;
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
        Ok(_) => req.rsp.take().expect("unable to get the rsp"),
        Err(ParkError::Timeout) => {
            // remove the req from req map
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
    // default timeout is 10s
    timeout: Duration,
    // the connection
    sock: Arc<UdpSocket>,
    // the waiting request
    req_map: Arc<WaitReqMap>,
    // the listening coroutine
    listener: Option<coroutine::JoinHandle<()>>,
}

unsafe impl Send for UdpClient {}
unsafe impl Sync for UdpClient {}

impl Drop for UdpClient {
    fn drop(&mut self) {
        self.listener.take().map(|h| {
            unsafe { h.coroutine().cancel() };
            h.join().ok();
        });
    }
}

impl UdpClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<UdpClient> {
        // this would bind a random port by the system
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.connect(addr)?;
        let sock = Arc::new(sock);
        let req_map = Arc::new(WaitReqMap::new());

        let sock_1 = sock.clone();
        let req_map_1 = req_map.clone();
        let listener = coroutine::Builder::new().name("UdpClientListener".to_owned())
            .spawn(move || {
                let mut buf = Vec::with_capacity(1024);
                buf.resize(1024, 0);
                loop {
                    t!(sock_1.recv(&mut buf));

                    // deserialize the rsp
                    let rsp: Response = t!(bincode::serde::deserialize(&buf));
                    info!("receive rsp, id={}", rsp.id);

                    // get the wait req
                    req_map_1.get(rsp.id).map(move |req| {
                        // set the response
                        req.rsp = Some(rsp.data.map_err(Error::from));
                        // wake up the blocker
                        req.blocker.unpark();
                    });
                }
            })?;

        Ok(UdpClient {
            id: AtomicUsize::new(0),
            timeout: Duration::from_secs(10),
            sock: sock,
            req_map: req_map,
            listener: Some(listener),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 10 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    // wait for response
    fn wait_rsp(&self, id: usize) -> Result<Vec<u8>, Error> {
        wait_rsp(&self.req_map, id, self.timeout)
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

        // wait for the rsp
        self.wait_rsp(id)
    }
}
