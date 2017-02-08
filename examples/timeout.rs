extern crate conetty;
extern crate coroutine;
extern crate env_logger;

use std::str;
use std::io::Write;
use std::time::Duration;
use conetty::{Server, Client, WireError, UdpServer, UdpClient, FrameBuf, RspBuf};

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        println!("req = {:?}", req);
        coroutine::sleep(Duration::from_secs(1));
        // rsp.write_all(req).map_err(|e| WireError::ServerSerialize(e.to_string()))
        Err(WireError::ServerDeserialize("asfasfdasd".into()))
    }
}

fn main() {
    env_logger::init().unwrap();

    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let mut client = UdpClient::connect(addr).unwrap();

    client.set_timeout(Duration::from_millis(500));
    let mut req = FrameBuf::new();
    write!(req, "aaaaaa").unwrap();
    println!("rsp = {:?}", client.call_service(req));

    client.set_timeout(Duration::from_millis(1500));
    let mut req = FrameBuf::new();
    write!(req, "bbbbbb").unwrap();
    println!("rsp = {:?}", client.call_service(req));

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
