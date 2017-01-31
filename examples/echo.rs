extern crate conetty;
extern crate coroutine;
extern crate env_logger;
extern crate bincode;
#[macro_use]
extern crate serde_derive;

use std::str;
use bincode::serde as encode;
use bincode::SizeLimit::Infinite;
// use serde::{Serialize, Deserialize};
use conetty::{Service, Error, WireError, UdpServer, UdpClient};

struct Echo;

// rpc spec
trait EchoRpc {
    fn echo(&self, data: String) -> String;
}

// server implementation
impl EchoRpc for Echo {
    fn echo(&self, data: String) -> String {
        data
    }
}

// ------------------------------------------------------------------------------------------------
// below should be generated code
#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
enum EchoRpcEnum {
    __invalid(()),
    hello(String),
}

trait EchoRpcClient {
    fn echo(&self, data: String) -> Result<String, Error>;
}

impl EchoRpcClient for UdpClient {
    fn echo(&self, data: String) -> Result<String, Error> {
        let mut buf = Vec::with_capacity(1024);
        // serialize the para
        encode::serialize_into(&mut buf, &data, Infinite)
            .map_err(|e| Error::ClientSerialize(e.to_string()))?;
        // call the server
        let ret = self.call_service(&buf)?;
        // deserialized the response
        encode::deserialize(&ret).map_err(|e| Error::ClientDeserialize(e.to_string()))
    }
}

impl Service for Echo {
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError> {
        // deserialize the request
        let data = encode::deserialize(request)
            .map_err(|e| WireError::ServerDeserialize(e.to_string()))?;
        // call the service
        let ret = self.echo(data);
        // serialize the result
        let mut buf = Vec::with_capacity(1024);
        encode::serialize_into(&mut buf, &ret, Infinite)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))?;
        // send the response
        Ok(buf)
    }
}
// ------------------------------------------------------------------------------------------------

fn main() {
    env_logger::init().unwrap();

    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let client = UdpClient::connect(addr).unwrap();

    for i in 0..10 {
        let s = format!("Hello World! id={}", i);
        let data = client.echo(s);
        println!("recv = {:?}", data);
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
