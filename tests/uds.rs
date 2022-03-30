#![cfg(unix)]

use std::io::Write;
use std::time::Duration;

use conetty::{SimpleClient, ReqBuf, RspBuf, Server, UdsClient, UdsServer, WireError};
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
    let path = "/tmp/test_uds";
    let server = Echo.start(path).unwrap();
    let mut client = UdsClient::connect(path).unwrap();

    let mut req = ReqBuf::new();
    req.write(&vec![5u8; 16]).unwrap();
    let rsp_frame = client.call_service(req).unwrap();
    let rsp = rsp_frame.decode_rsp().unwrap();
    assert_eq!(rsp, &[5u8; 16]);

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}

#[test]
fn uds_timeout() {
    struct Echo;

    impl Server for Echo {
        fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
            coroutine::sleep(Duration::from_secs(1));
            rsp.write_all(req)
                .map_err(|e| WireError::ServerSerialize(e.to_string()))
        }
    }

    let path = "/tmp/test_uds1";
    let server = Echo.start(path).unwrap();
    let mut client = UdsClient::connect(path).unwrap();

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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let path = "/tmp/test_uds2";
    let server = Echo.start(path).unwrap();

    let count = Arc::new(AtomicUsize::new(0));

    let mut vec = vec![];
    for i in 0..8 {
        let count_ref = count.clone();
        let h = go!(move || {
            let mut client = UdsClient::connect(path).unwrap();
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
