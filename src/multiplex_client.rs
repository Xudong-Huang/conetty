use std::io::{self, BufReader};
use std::net::ToSocketAddrs;
use std::time::Duration;

use crate::errors::Error;
use crate::frame::{Frame, ReqBuf};
use crate::queued_writer::QueuedWriter;
use crate::Client;
use co_waiter::token_waiter::TokenWaiter;
use may::net::TcpStream;
use may::{coroutine, go};

#[derive(Debug)]
pub struct MultiplexClient {
    // default timeout is 10s
    timeout: Option<Duration>,
    // the connection
    sock: QueuedWriter<TcpStream>,
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

        if let Some(h) = self.listener.take() {
            unsafe { h.coroutine().cancel() };
            h.join().ok();
        }
    }
}

impl MultiplexClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<MultiplexClient> {
        // this is a client side server that listening from server!
        let sock = TcpStream::connect(addr)?;

        // here we must clone the socket for read
        // we can't share it between coroutines
        let sock1 = sock.try_clone()?;
        let mut r_stream = BufReader::new(sock1);
        let listener = go!(
            coroutine::Builder::new().name("MultiPlexClientListener".to_owned()),
            move || {
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
                    let id = rsp_frame.id as usize;
                    // ignore the cases that failed to find out a req waiter
                    TokenWaiter::set_rsp(id, rsp_frame);
                    // .unwrap_or_else(|_| panic!("failed to set rsp: id={}", id));
                }
            }
        )?;

        Ok(MultiplexClient {
            timeout: None,
            sock: QueuedWriter::new(sock),
            listener: Some(listener),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 10 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }
}

impl Client for MultiplexClient {
    fn call_service(&self, req: ReqBuf) -> Result<Frame, Error> {
        let waiter = TokenWaiter::new();
        let waiter = std::pin::Pin::new(&waiter);
        let id = waiter.get_id();
        info!("request id = {}", id);

        // send the request
        let buf = req.finish(id as u64);

        self.sock.write(buf);

        // wait for the rsp
        Ok(waiter.wait_rsp(self.timeout)?)
    }
}
