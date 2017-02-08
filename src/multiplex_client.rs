use std::sync::Arc;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::collections::HashMap;
use std::io::{self, BufReader, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use Client;
use coroutine;
use errors::Error;
use frame::{Frame, ReqBuf};
use coroutine::net::TcpStream;
use coroutine::sync::{AtomicOption, Mutex, Blocker};

struct WaitReq {
    blocker: Blocker,
    rsp: AtomicOption<Frame>,
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
            -> Result<Frame, Error> {
    use coroutine::ParkError;

    match req.blocker.park(Some(timeout)) {
        Ok(_) => {
            match req.rsp.take(Ordering::Acquire) {
                Some(frame) => Ok(frame),
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

pub struct MultiplexClient {
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

unsafe impl Send for MultiplexClient {}
unsafe impl Sync for MultiplexClient {}

impl Drop for MultiplexClient {
    fn drop(&mut self) {
        self.listener.take().map(|h| {
            unsafe { h.coroutine().cancel() };
            h.join().ok();
        });
    }
}

impl MultiplexClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<MultiplexClient> {
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
                    let rsp_frame = match Frame::decode_from(&mut r_stream) {
                        Ok(r) => r,
                        Err(ref e) => {
                            if e.kind() == io::ErrorKind::UnexpectedEof {
                                info!("tcp multiplex_client decode rsp: connection closed");
                            } else {
                                error!("tcp multiplex_client decode rsp: err = {:?}", e);
                            }
                            break;
                        }
                    };
                    info!("receive rsp, id={}", rsp_frame.id);

                    // get the wait req
                    req_map_1.get(rsp_frame.id as usize).map(move |req| {
                        // set the response
                        req.rsp.swap(rsp_frame, Ordering::Release);
                        // wake up the blocker
                        req.blocker.unpark();
                    });
                }
            })?;

        Ok(MultiplexClient {
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

impl Client for MultiplexClient {
    fn call_service(&self, req: ReqBuf) -> Result<Frame, Error> {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        info!("request id = {}", id);

        // must regiser before io
        let cur = Blocker::new(false);
        let mut rw = WaitReq {
            blocker: cur,
            rsp: AtomicOption::none(),
        };
        self.req_map.add(id, &mut rw);

        // send the request
        let buf = req.finish(id as u64);

        let mut g = self.sock.lock().unwrap();
        g.write_all(&buf).map_err(Error::from)?;
        drop(g);

        // wait for the rsp
        wait_rsp(&self.req_map, id, self.timeout, &mut rw)
    }
}
