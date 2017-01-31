use std::sync::Arc;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::io::{self, BufReader, Write};
use Client;
use coroutine;
use io::Response;
use errors::Error;
use bincode::serde as encode;
use coroutine::net::TcpStream;
use bincode::SizeLimit::Infinite;
use bincode::serde::DeserializeError::IoError;
use coroutine::sync::{AtomicOption, Mutex, Blocker};

struct WaitReq {
    blocker: Blocker,
    rsp: AtomicOption<Result<Vec<u8>, Error>>,
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
fn wait_rsp(req_map: &WaitReqMap,
            id: usize,
            timeout: Duration,
            req: &mut WaitReq)
            -> Result<Vec<u8>, Error> {
    use coroutine::ParkError;

    match req.blocker.park(Some(timeout)) {
        Ok(_) => {
            match req.rsp.take(Ordering::Acquire) {
                Some(d) => d,
                None => panic!("unable to get the rsp, id={}", id),
            }
        }
        Err(ParkError::Timeout) => {
            // remove the req from req map
            error!("timeout zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz, id={}", id);
            req_map.get(id);
            Err(Error::Timeout)
        }
        Err(ParkError::Canceled) => {
            error!("canceled id={}", id);
            req_map.get(id);
            coroutine::trigger_cancel_panic();
        }
    }
}

pub struct MultiPlexClient {
    // each request would have a unique id
    id: AtomicUsize,
    // default timeout is 10s
    timeout: Duration,
    // the connection
    sock: Mutex<TcpStream>,
    // the waiting request
    req_map: Arc<WaitReqMap>,
    // the listening coroutine
    listener: Option<coroutine::JoinHandle<()>>,
}

unsafe impl Send for MultiPlexClient {}
unsafe impl Sync for MultiPlexClient {}

impl Drop for MultiPlexClient {
    fn drop(&mut self) {
        self.listener.take().map(|h| {
            unsafe { h.coroutine().cancel() };
            h.join().ok();
        });
    }
}

impl MultiPlexClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<MultiPlexClient> {
        let sock = TcpStream::connect(addr)?;
        let req_map = Arc::new(WaitReqMap::new());

        // here we must clone the socket for read
        // we can't share it between coroutines
        let sock1 = sock.try_clone()?;
        let mut r_stream = BufReader::new(sock1);
        let req_map_1 = req_map.clone();
        let listener = coroutine::Builder::new().name("MultiPlexClientListener".to_owned())
            .spawn(move || {
                loop {
                    let rsp: Response = match encode::deserialize_from(&mut r_stream, Infinite) {
                        Ok(r) => r,
                        Err(IoError(ref e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                            info!("connection is closed");
                            break;
                        }
                        Err(ref e) => {
                            error!("deserialize_from err={:?}", e);
                            continue;
                        }
                    };
                    info!("receive rsp, id={}", rsp.id);

                    // get the wait req
                    req_map_1.get(rsp.id).map(move |req| {
                        // set the response
                        // req.rsp = Some(rsp.data.map_err(Error::from));
                        req.rsp.swap(rsp.data.map_err(Error::from), Ordering::Release);
                        // wake up the blocker
                        req.blocker.unpark();
                    });
                }
            })?;

        Ok(MultiPlexClient {
            id: AtomicUsize::new(0),
            timeout: Duration::from_secs(10),
            sock: Mutex::new(sock),
            req_map: req_map,
            listener: Some(listener),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 10 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

impl Client for MultiPlexClient {
    fn call_service(&self, req: &[u8]) -> Result<Vec<u8>, Error> {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        info!("request id = {}", id);

        // must regiser before io
        let cur = Blocker::new(false);
        let mut rw = WaitReq {
            blocker: cur,
            rsp: AtomicOption::none(),
        };
        self.req_map.add(id, &mut rw);

        let mut buf = Vec::with_capacity(1024);
        // serialize the request id
        encode::serialize_into(&mut buf, &id, Infinite)
        .map_err(|e| Error::ClientSerialize(e.to_string()))?;
        // serialize the request
        encode::serialize_into(&mut buf, &req, Infinite)
        .map_err(|e| Error::ClientSerialize(e.to_string()))?;

        let mut g = self.sock.lock().unwrap();
        g.write(&buf).map_err(Error::from)?;
        drop(g);

        // wait for the rsp
        wait_rsp(&self.req_map, id, self.timeout, &mut rw)
    }
}
