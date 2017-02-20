## conetty: Rust coroutine based client/server framework
[![Software License](https://img.shields.io/badge/license-MIT-brightgreen.svg)](LICENSE)

conetty is a general CS framework for rust based on coroutines with a focus on ease of use.

the general communication procedure is as below
1. client send request to server
2. server recv request from client
3. server parsing and process the request
4. server send out response to client
5. client recv response from server

this lib provide a general request/response struct called `Frame`, it's just a wrapper for the raw
data `Vec<u8>`. you need to prepare and parsing it in the actual process functions that passed into
the framework

[Documentation](https://docs.rs/conetty)

## Usage

Add to your `Cargo.toml` dependencies:

```toml
conetty = { git = "https://github.com/Xudong-Huang/conetty" }
```

## Example

```rust
extern crate conetty;

use std::str;
use std::io::Write;
use conetty::{Server, Client, WireError, TcpServer, TcpClient, ReqBuf, RspBuf};

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        println!("req = {:?}", req);
        rsp.write_all(req).map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

fn main() {
    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let client = TcpClient::connect(addr).unwrap();

    for i in 0..10 {
        let mut buf = ReqBuf::new();
        buf.write_fmt(format_args!("Hello World! id={}", i)).unwrap();
        let data = client.call_service(buf).unwrap();
        let rsp = data.decode_rsp().unwrap();
        println!("recv = {:?}", str::from_utf8(&rsp).unwrap());
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
```

## Additional Features
- Mulitiplex for a single connection
- support TCP/UDP
- Run any number of clients and services

## License

conetty is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
