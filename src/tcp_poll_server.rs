use std::sync::Arc;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::io::{self, Cursor, BufReader, Write};
use Server;
use coroutine;
use errors::WireError;
use comanaged::Manager;
use frame::{Frame, RspBuf};
use coroutine::sync::Mutex;
use coroutine::net::TcpListener;
use byteorder::ReadBytesExt;

/// Provides a function for starting the tcp polling service.
pub trait TcpPollServer: Server {
    /// Spawns the service, binding to the given address
    /// return a coroutine that you can cancel it when need to stop the service
    fn start<L: ToSocketAddrs>(self, addr: L) -> io::Result<coroutine::JoinHandle<()>> {
        let listener = TcpListener::bind(addr)?;
        coroutine::Builder::new().name("TcpServer".to_owned()).spawn(move || {
            let server = Arc::new(self);
            let manager = Manager::new();
            for stream in listener.incoming() {
                let stream = t!(stream);
                let server = server.clone();
                manager.add(move |_| {
                    let rs = stream.try_clone().expect("failed to clone stream");
                    // every 15s we do a timeout check, if client is not respose we
                    // think the client is down and need to finish the coroutine
                    rs.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
                    // the read half of the stream
                    let mut rs = BufReader::new(rs);
                    // the write half need to be protected by mutex
                    // for that coroutine io obj can't shared safely
                    let ws = Arc::new(Mutex::new(stream));

                    loop {
                        // first send out the poll request, client will return back the req frames
                        // if there is no data, client will send back with no frames and this is
                        // some form of heart beat check and also we have a changce to clean up the
                        // bad connections
                        let rsp = RspBuf::new();
                        // the req polling id is always 0
                        let data = rsp.finish(0, Err(WireError::Polling));
                        info!("send req polling request");
                        match ws.lock().unwrap().write_all(&data) {
                            Ok(_) => {}
                            Err(e) => {
                                error!("send req polling: err={:?}", e);
                                break; // finish the coroutine
                            }
                        }

                        let reqs = match Frame::decode_from(&mut rs) {
                            Ok(r) => r,
                            Err(ref e) => {
                                if e.kind() == io::ErrorKind::UnexpectedEof {
                                    warn!("tcp server decode req: connection closed");
                                } else {
                                    error!("tcp server decode req: err = {:?}", e);
                                }
                                break;
                            }
                        };

                        assert_eq!(reqs.id, 0);
                        let mut reqs = Cursor::new(reqs.decode_req());

                        // frames encodes as: frames(u8) + frame0 + frame1 + ...
                        // with max frames 255, max total length 1024*1023
                        let frames = reqs.read_u8().expect("can't read out frame numbers");
                        for _ in 0..frames {
                            let req = match Frame::decode_from(&mut reqs) {
                                Ok(r) => r,
                                Err(ref e) => {
                                    error!("tcp poll server decode req: err = {:?}", e);
                                    break;
                                }
                            };

                            info!("get request: id={:?}", req.id);
                            let w_stream = ws.clone();
                            let server = server.clone();
                            coroutine::spawn(move || {
                                let mut rsp = RspBuf::new();
                                let ret = server.service(req.decode_req(), &mut rsp);
                                let data = rsp.finish(req.id, ret);

                                info!("send rsp: id={}", req.id);
                                // send the result back to client
                                w_stream.lock()
                                    .unwrap()
                                    .write_all(&data)
                                    .unwrap_or_else(|e| error!("send rsp failed: err={:?}", e));
                            });
                        }
                    }
                });
            }
        })
    }
}

impl<T: Server> TcpPollServer for T {}
