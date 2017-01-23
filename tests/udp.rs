extern crate conetty;
extern crate coroutine;

use std::str;
use std::time::Duration;
use conetty::{Service, WireError, UdpServer, UdpClient};

struct Echo;

impl Service for Echo {
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError> {
        Ok(request.to_vec())
    }
}

#[test]
fn echo() {
    let addr = ("127.0.0.1", 2000);
    let server = Echo.start(&addr).unwrap();
    let client = UdpClient::connect(addr).unwrap();

    let rsp = client.call_service(&vec![5; 80]).unwrap();
    assert_eq!(rsp, vec![5; 80]);

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}

#[test]
fn tcp_timeout() {
    struct Echo;

    impl Service for Echo {
        fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError> {
            coroutine::sleep(Duration::from_secs(1));
            Ok(request.to_vec())
        }
    }


    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let mut client = UdpClient::connect(addr).unwrap();

    client.set_timeout(Duration::from_millis(500));
    assert!(client.call_service("aaaaaa".as_bytes()).is_err());

    client.set_timeout(Duration::from_millis(1500));
    assert!(client.call_service("bbbbbb".as_bytes()).is_ok());

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
        let h = coroutine::spawn(move || {
            let client = UdpClient::connect(addr).unwrap();
            for j in 0..10 {
                let s = format!("Hello World! id={}, j={}", i, j);
                match client.call_service(s.as_bytes()) {
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
