#![feature(test)]
extern crate test;
extern crate conetty;

use std::io::Write;
use test::Bencher;
use conetty::{Server, Client, WireError, TcpServer, TcpClient, ReqBuf, RspBuf};

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        rsp.write_all(req)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

#[bench]
fn tcp_echo(b: &mut Bencher) {
    conetty::may::config()
        .set_workers(2)
        .set_io_workers(4)
        .set_pool_capacity(1000);
    let addr = ("127.0.0.1", 2000);
    let server = Echo.start(&addr).unwrap();
    let client = TcpClient::connect(addr).unwrap();

    b.iter(|| {
               let mut req = ReqBuf::new();
               req.write(&vec![0; 100]).unwrap();
               let _rsp = client.call_service(req).unwrap();
           });

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
