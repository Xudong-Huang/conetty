use std::io::Write;
use std::time::Duration;

use conetty::{Client, ReqBuf, RspBuf, Server, TcpClient, TcpServer, WireError};
use may::{coroutine, go};

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        rsp.write_all(req)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

#[test]
fn echo() {
    let addr = ("127.0.0.1", 2000);
    let server = Echo.start(&addr).unwrap();
    let client = TcpClient::connect(addr).unwrap();

    let mut req = ReqBuf::new();
    req.write(&vec![5u8; 16]).unwrap();
    let rsp_frame = client.call_service(req).unwrap();
    let rsp = rsp_frame.decode_rsp().unwrap();
    assert_eq!(rsp, &[5u8; 16]);

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}

#[test]
fn tcp_timeout() {
    struct Echo;

    impl Server for Echo {
        fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
            coroutine::sleep(Duration::from_secs(1));
            rsp.write_all(req)
                .map_err(|e| WireError::ServerSerialize(e.to_string()))
        }
    }

    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let mut client = TcpClient::connect(addr).unwrap();

    client.set_timeout(Duration::from_millis(500));
    let mut req = ReqBuf::new();
    write!(req, "aaaaaa").unwrap();
    assert!(client.call_service(req).is_err());

    client.set_timeout(Duration::from_millis(1500));
    let mut req = ReqBuf::new();
    write!(req, "bbbbbb").unwrap();
    assert!(client.call_service(req).is_ok());

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}

#[test]
fn multi_client() {
    use std::mem;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let addr = ("127.0.0.1", 3000);
    let server = Echo.start(&addr).unwrap();

    let count = AtomicUsize::new(0);

    let mut vec = vec![];
    for i in 0..8 {
        let count_ref: &'static AtomicUsize = unsafe { mem::transmute(&count) };
        let h = go!(move || {
            let client = TcpClient::connect(addr).unwrap();
            for j in 0..10 {
                let mut req = ReqBuf::new();
                write!(req, "Hello World! id={}, j={}", i, j).unwrap();
                match client.call_service(req) {
                    Ok(_) => {
                        count_ref.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(err) => panic!("recv err = {:?}", err),
                }
            }
        });
        vec.push(h);
    }

    for (_i, j) in vec.into_iter().enumerate() {
        j.join().unwrap();
    }

    assert_eq!(count.load(Ordering::Relaxed), 80);

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
