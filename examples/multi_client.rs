use std::io::Write;

use conetty::{Client, ReqBuf, RspBuf, Server, TcpClient, TcpServer, WireError};
use may::go;

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        // println!("req = {:?}", req);
        rsp.write_all(req)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

fn main() {
    env_logger::init();
    may::config().set_workers(4);

    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();

    let mut vec = vec![];
    for i in 0..100 {
        let handle = go!(move || {
            let client = TcpClient::connect(addr).unwrap();
            for j in 0..1000 {
                let mut req = ReqBuf::new();
                write!(req, "Hello World! id={}, j={}", i, j).unwrap();
                match client.call_service(req) {
                    // Ok(frame) => {
                    //     let rsp = frame.decode_rsp().unwrap();
                    //     println!("recv = {:?}", str::from_utf8(rsp).unwrap());
                    // }
                    Err(err) => println!("recv err = {:?}", err),
                    _ => {}
                }
            }
            // ::std::mem::forget(client);
            println!("thread done, id={}", i);
        });
        vec.push(handle);
    }

    for (i, j) in vec.into_iter().enumerate() {
        j.join().unwrap();
        println!("wait for {} done", i);
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
