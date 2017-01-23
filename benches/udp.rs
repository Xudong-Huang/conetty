#![feature(test)]
extern crate test;
extern crate conetty;
extern crate coroutine;

use test::Bencher;
use conetty::{Service, WireError, UdpServer, UdpClient};

struct Echo;

impl Service for Echo {
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError> {
        Ok(request.to_vec())
    }
}

#[bench]
fn udp_echo(b: &mut Bencher) {
    let addr = ("127.0.0.1", 3000);
    let server = Echo.start(&addr).unwrap();
    let client = UdpClient::connect(addr).unwrap();

    b.iter(|| {
        let _rsp = client.call_service(&vec![0; 100]).unwrap();
    });

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
