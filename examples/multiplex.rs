use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use conetty::{Client, MultiplexClient, ReqBuf, RspBuf, Server, TcpServer, WireError};
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

    let addr = ("127.0.0.1", 4000);
    let _server = Echo.start(&addr).unwrap();

    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let mut client = MultiplexClient::new(tcp_stream).unwrap();
    client.set_timeout(Duration::from_secs(5));

    let client = Arc::new(client);
    let mut vec = vec![];
    for i in 0..100 {
        let client = client.clone();
        let j = go!(move || {
            for j in 0..1000 {
                let mut req = ReqBuf::new();
                write!(req, "Hello World! id={}, j={}", i, j).unwrap();
                match client.call_service(req) {
                    Err(err) => println!("recv err = {:?}", err),
                    Ok(frame) => {
                        let rsp = frame.decode_rsp().unwrap();
                        log::info!("recv = {:?}", std::str::from_utf8(rsp).unwrap());
                    }
                }
            }
            println!("thread done, id={}", i);
        });
        vec.push(j);
    }

    for (i, j) in vec.into_iter().enumerate() {
        j.join().unwrap();
        println!("wait for {} done", i);
    }
}
