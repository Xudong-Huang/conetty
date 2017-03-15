extern crate conetty;
extern crate coroutine;
extern crate env_logger;

use std::str;
use std::io::Write;
use std::time::Duration;
use conetty::{Server, Client, WireError, UdpServer, UdpClient, ReqBuf, RspBuf};

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], _rsp: &mut RspBuf) -> Result<(), WireError> {
        println!("req = {:?}", req);
        coroutine::sleep(Duration::from_secs(1));
        // rsp.write_all(req).map_err(|e| WireError::ServerSerialize(e.to_string()))
        // Err(WireError::ServerDeserialize("asfasfdasd".into()))
        Err(WireError::Status("timeout".to_owned()))
    }
}

fn main() {
    env_logger::init().unwrap();

    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let mut client = UdpClient::connect(addr).unwrap();

    client.set_timeout(Duration::from_millis(500));
    let mut req = ReqBuf::new();
    write!(req, "aaaaaa").unwrap();
    println!("rsp = {:?}", client.call_service(req));

    client.set_timeout(Duration::from_millis(1500));
    let mut req = ReqBuf::new();
    write!(req, "bbbbbb").unwrap();
    let rsp_frame = client.call_service(req).unwrap();
    println!("rsp_frame = {:?}", rsp_frame);
    let rsp = rsp_frame.decode_rsp();
    println!("rsp = {:?}", rsp);

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
