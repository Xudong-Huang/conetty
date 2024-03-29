use std::io::{self, BufReader};
use std::time::Duration;

use crate::errors::Error;
use crate::frame::{Frame, ReqBuf};
use crate::stream_ext::StreamExt;

pub struct StreamClient<S: StreamExt> {
    // each request would have a unique id
    id: u64,
    // the connection
    stream: BufReader<S>,
}

impl<S: StreamExt> StreamClient<S> {
    /// connect to the server address
    pub fn new(stream: S) -> Self {
        StreamClient {
            id: 0,
            stream: BufReader::with_capacity(1024, stream),
        }
    }
}

impl<S: StreamExt> StreamClient<S> {
    /// set timeout
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), io::Error> {
        self.stream.get_mut().set_read_timeout(timeout)
    }
}

impl<S: StreamExt> StreamClient<S> {
    /// call the server
    /// the request must be encoded into the ReqBuf
    /// the response is the raw frame, you should parsing it into final response
    pub fn call_service(&mut self, req: ReqBuf) -> Result<Frame, Error> {
        let id = self.id;
        self.id += 1;
        info!("request id = {}", id);

        // encode the request
        self.stream.get_mut().write_all(&(req.finish(id)))?;

        // read the response
        loop {
            // deserialize the rsp
            let rsp_frame = Frame::decode_from(&mut self.stream)
                .map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // discard the rsp that is is not belong to us
            if rsp_frame.id == id {
                info!("get response id = {}", id);
                return Ok(rsp_frame);
            }
        }
    }
}
