#![feature(test)]
extern crate test;

use std::io::Write;

use conetty::{ReqBuf, RspBuf, Server, StreamClient, UdsServer, WireError};
use test::Bencher;

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        rsp.write_all(req)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

#[bench]
fn uds_echo(b: &mut Bencher) {
    may::config().set_workers(2).set_pool_capacity(1000);
    let addr = "/tmp/test_uds";
    let _server = Echo.start(&addr).unwrap();
    let tcp_stream = may::os::unix::net::UnixStream::connect(addr).unwrap();
    let mut client = StreamClient::new(tcp_stream);

    b.iter(|| {
        let mut req = ReqBuf::new();
        req.write_all(&[0; 100]).unwrap();
        let _rsp = client.call_service(req).unwrap();
    });
}
