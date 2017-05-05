use std::sync::Arc;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::io::{self, BufReader, Write};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::atomic::{AtomicUsize, Ordering};
use Client;
use coroutine;
use errors::Error;
use frame::{Frame, ReqBuf};
use coroutine::net::TcpStream;
use byteorder::{BigEndian, WriteBytesExt};
use coroutine::sync::mpmc::{channel, Sender};
use wait_req::{WaitReq, WaitReqMap, wait_rsp};
use coroutine::sync::{AtomicOption, Mutex, Blocker};

pub struct TcpPollClient {
    // each request would have a unique id
    id: AtomicUsize,
    // default timeout is 10s
    timeout: Duration,
    // the connection
    // sock: Mutex<TcpStream>,
    // the waiting request
    req_map: Arc<WaitReqMap>,
    // the listening coroutine
    listener: Option<coroutine::JoinHandle<()>>,
    // the request queue
    req_queue_tx: Sender<Vec<u8>>,
}

unsafe impl Send for TcpPollClient {}
unsafe impl Sync for TcpPollClient {}

impl Drop for TcpPollClient {
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

impl TcpPollClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<TcpPollClient> {
        // this is a client side server that listening from server!
        let sock = TcpStream::connect(addr)?;
        let req_map = Arc::new(WaitReqMap::new());
        let (tx, rx) = channel::<Vec<u8>>();

        // here we must clone the socket for read
        // we can't share it between coroutines
        let sock1 = sock.try_clone()?;
        let mut r_stream = BufReader::new(sock1);
        let req_map_1 = req_map.clone();
        let listener = coroutine::Builder::new().name("MultiPlexClientListener".to_owned())
            .spawn(move || {
                // the write part of the socket
                let w_sock =  Mutex::new(sock);
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
                    // if we receive a rsp frame with id=0 and ty=Polling
                    // then this is a server poll request
                    // start a coroutine to process it
                    if rsp_frame.id == 0
                    // && ty =200
                    {
                        let rx = rx.clone();
                        coroutine::spawn(move || {
                            // try to collect as much req frame as possible
                            let mut frames = 0;
                            let mut total_len = 0;
                            let mut req_buf = ReqBuf::new();
                            // first write a fake frames
                            req_buf.write_u8(0).expect("failed to write frame number");
                            loop {
                                match rx.try_recv() {
                                    Ok(req_frame) => {
                                        // append the req_frame
                                        req_buf.write_all(&req_frame).expect("failed to write frame");
                                        frames += 1;
                                        if frames >= 255 {
                                            break;
                                        }
                                        if total_len >= 1024 * 1023 {
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                                // if the req frames is not collected we just wait for 10s
                                if frames == 0 {
                                    match rx.recv_timeout(Duration::from_secs(10)) {
                                        Ok(req_frame) => {collect_frames()}
                                        Err(RecvTimeoutError::Timeout) => {
                                            // timeout happened so we still send out the empty frames
                                            return send_frames();
                                        }
                                        _ => unreachable!("client collect req error!"),
                                    }
                                }
                                return send_frames();
                            }
                        });
                    }

                    // get the wait req
                    req_map_1.get(rsp_frame.id as usize).map(move |req| {
                        // set the response
                        req.rsp.swap(rsp_frame, Ordering::Release);
                        // wake up the blocker
                        req.blocker.unpark();
                    });
                }
            })?;

        Ok(TcpPollClient {
            id: AtomicUsize::new(0),
            timeout: Duration::from_secs(10),
            // sock: Mutex::new(sock),
            req_map: req_map,
            listener: Some(listener),
            req_queue_tx: tx,
        })
    }

    /// set the default timeout value
    /// the initial timeout is 10 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

impl Client for TcpPollClient {
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

        self.req_queue_tx.send(buf).expect("failed to send req frame to client queue");

        // let mut g = self.sock.lock().unwrap();
        // g.write_all(&buf)
        //     .map_err(|err| {
        //         // pop out the wait req if failed to send
        //         self.req_map.get(id);
        //         Error::from(err)
        //     })?;
        // drop(g);

        // wait for the rsp
        wait_rsp(&self.req_map, id, self.timeout, &mut rw)
    }
}
