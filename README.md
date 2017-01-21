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
tarpc = { git = "https://github.com/Xudong-Huang/conetty" }
```

## Example

```rust
#[macro_use]
extern crate conetty;

use conetty::{client, server};

rpc_service! {
    rpc hello(name: String) -> String;
}

#[derive(Clone)]
struct HelloServer;

impl RpcService for HelloServer {
    fn hello(&self, name: String) -> String {
        format!("Hello, {}!", name)
    }
}

fn main() {
    let addr = "localhost:10000";
    HelloServer.listen(addr).unwrap();
    let client = RpcClient::connect(addr).unwrap();
    println!("{}", client.hello("Mom".to_string()).unwrap());
}
```

## Additional Features
- Mulitiplex for a single connection
- support TCP/UDP
- Run any number of clients and services

## License

conetty is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
