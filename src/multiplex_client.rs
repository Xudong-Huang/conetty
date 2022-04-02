use std::io::{self, BufReader, Read, Write};
use std::time::Duration;

use crate::errors::Error;
use crate::frame::{Frame, ReqBuf};
use crate::queued_writer::QueuedWriter;
use crate::Client;

use may::{coroutine, go};
use may_waiter::TokenWaiter;

pub trait TryClone: Sized + Send + 'static {
    fn try_clone(&self) -> Result<Self, io::Error>;
}

macro_rules! impl_try_clone {
    ($name: ty) => {
        impl TryClone for $name {
            fn try_clone(&self) -> Result<Self, io::Error> {
                self.try_clone()
            }
        }
    };
}

impl_try_clone!(std::net::TcpStream);
impl_try_clone!(may::net::TcpStream);
#[cfg(unix)]
impl_try_clone!(std::os::unix::net::UnixStream);
#[cfg(unix)]
impl_try_clone!(may::os::unix::net::UnixStream);

#[derive(Debug)]
pub struct MultiplexClient<S: Read + Write> {
    // default timeout is 10s
    timeout: Option<Duration>,
    // the connection
    sock: QueuedWriter<S>,
    // the listening coroutine
    listener: Option<coroutine::JoinHandle<()>>,
}

impl<S: Read + Write> Drop for MultiplexClient<S> {
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

impl<S: Read + Write + TryClone> MultiplexClient<S> {
    /// connect to the server address
    pub fn new(stream: S) -> io::Result<Self> {
        // here we must clone the socket for read
        // we can't share it between coroutines
        let stream1 = stream.try_clone()?;
        let mut r_stream = BufReader::new(stream1);
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
                    let id = unsafe { may_waiter::ID::from_usize(rsp_frame.id as usize) };
                    TokenWaiter::set_rsp(id, rsp_frame);
                }
            }
        )?;

        Ok(MultiplexClient {
            timeout: None,
            sock: QueuedWriter::new(stream),
            listener: Some(listener),
        })
    }

    /// set the default timeout value
    /// the initial timeout is 10 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = Some(timeout);
    }
}

impl<S: Read + Write> Client for MultiplexClient<S> {
    fn call_service(&self, req: ReqBuf) -> Result<Frame, Error> {
        let waiter = TokenWaiter::new();
        let id = waiter.id().unwrap();
        info!("request id = {:?}", id);

        // send the request
        let id: usize = id.into();
        let buf = req.finish(id as u64);

        self.sock.write(buf);

        // wait for the rsp
        Ok(waiter.wait_rsp(self.timeout)?)
    }
}
