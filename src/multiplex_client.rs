use std::sync::Arc;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::io::{self, BufReader, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use Client;
use coroutine;
use errors::Error;
use may::sync::Mutex;
use may::net::TcpStream;
use co_waiter::WaiterMap;
use frame::{Frame, ReqBuf};

#[derive(Debug)]
pub struct MultiplexClient {
    // each request would have a unique id
    id: AtomicUsize,
    // default timeout is 10s
    timeout: Duration,
    // the connection
    sock: Mutex<TcpStream>,
    // the waiting request
    req_map: Arc<WaiterMap<u64, Frame>>,
    // the listening coroutine
    listener: Option<coroutine::JoinHandle<()>>,
}

unsafe impl Send for MultiplexClient {}
unsafe impl Sync for MultiplexClient {}

impl Drop for MultiplexClient {
    fn drop(&mut self) {
        if ::std::thread::panicking() {
            return;
        }
        self.listener.take().map(|h| {
            unsafe { h.coroutine().cancel() };
            h.join().ok();
        });
    }
}

impl MultiplexClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<MultiplexClient> {
        // this is a client side server that listening from server!
        let sock = TcpStream::connect(addr)?;
        let req_map = Arc::new(WaiterMap::new());

        // here we must clone the socket for read
        // we can't share it between coroutines
        let sock1 = sock.try_clone()?;
        let mut r_stream = BufReader::new(sock1);
        let req_map_1 = req_map.clone();
        let listener = coroutine::Builder::new()
            .name("MultiPlexClientListener".to_owned())
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

                    // set the wait req
                    let id = rsp_frame.id;
                    // ignore the cases that failed to find out a req waiter
                    req_map_1.set_rsp(&id, rsp_frame).ok();
                    // .unwrap_or_else(|_| panic!("failed to set rsp: id={}", id));
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
        let id = self.id.fetch_add(1, Ordering::Relaxed) as u64;
        info!("request id = {}", id);

        // must regiser before io
        let rw = self.req_map.new_waiter(id);

        // send the request
        let buf = req.finish(id);

        let mut g = self.sock.lock().unwrap();
        g.write_all(&buf)?;
        drop(g);

        // wait for the rsp
        Ok(rw.wait_rsp(self.timeout)?)
    }
}
