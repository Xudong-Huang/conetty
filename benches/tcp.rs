#![feature(test)]
extern crate test;

use std::io::Write;

use conetty::{SimpleClient, ReqBuf, RspBuf, Server, TcpClient, TcpServer, WireError};
use test::Bencher;

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        rsp.write_all(req)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

#[bench]
fn tcp_echo(b: &mut Bencher) {
    may::config().set_workers(2).set_pool_capacity(1000);
    let addr = ("127.0.0.1", 2000);
    let server = Echo.start(&addr).unwrap();
    let mut client = TcpClient::connect(addr).unwrap();

    b.iter(|| {
        let mut req = ReqBuf::new();
        req.write(&vec![0; 100]).unwrap();
        let _rsp = client.call_service(req).unwrap();
    });

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
