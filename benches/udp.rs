#![feature(test)]
extern crate test;

use std::io::Write;

use conetty::{ReqBuf, RspBuf, Server, UdpClient, UdpServer, WireError};
use test::Bencher;

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        rsp.write_all(req)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

#[bench]
fn udp_echo(b: &mut Bencher) {
    let addr = ("127.0.0.1", 3000);
    let server = Echo.start(&addr).unwrap();
    let mut client = UdpClient::connect(addr).unwrap();

    b.iter(|| {
        let mut req = ReqBuf::new();
        req.write(&vec![0; 100]).unwrap();
        let _rsp = client.call_service(req).unwrap();
    });

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
