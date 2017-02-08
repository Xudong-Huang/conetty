extern crate conetty;
extern crate coroutine;

use std::str;
use conetty::{Server, Client, WireError, TcpServer, TcpClient, FrameBuf};


use std::io::{Write, Cursor};

struct Echo;

impl Server for Echo {
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError> {
        println!("req = {:?}", request);
        Ok(request.to_vec())
    }
}

fn main() {
    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let client = TcpClient::connect(addr).unwrap();

    for i in 0..10 {
        let mut buf = FrameBuf::new();
        buf.write_fmt(format_args!("Hello World! id={}", i)).unwrap();
        let data = client.call_service(buf).unwrap();
        let rsp = data.decode_rsp().unwrap();
        println!("recv = {:?}", str::from_utf8(&rsp).unwrap());
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();


    let buf = vec![0; 10];
    let mut writer = Cursor::new(buf);
    writer.write("hello world!".as_bytes()).unwrap();
    let buf = writer.into_inner();
    println!("buf = {:?}\n cap={}", buf, buf.capacity());
}
