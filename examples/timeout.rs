extern crate conetty;
extern crate coroutine;
extern crate env_logger;

use std::str;
use std::time::Duration;
use conetty::{Server, Client, WireError, UdpServer, UdpClient};

struct Echo;

impl Server for Echo {
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError> {
        println!("req = {:?}", request);
        coroutine::sleep(Duration::from_secs(1));
        Ok(request.to_vec())
    }
}

fn main() {
    env_logger::init().unwrap();

    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let mut client = UdpClient::connect(addr).unwrap();

    client.set_timeout(Duration::from_millis(500));
    println!("rsp = {:?}", client.call_service("aaaaaa".as_bytes()));

    client.set_timeout(Duration::from_millis(1500));
    println!("rsp = {:?}", client.call_service("bbbbbb".as_bytes()));

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
