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
    fn add(&self, x: u32, y: u32) -> u32;
}

// server implementation
impl EchoRpc for Echo {
    fn echo(&self, data: String) -> String {
        data
    }

    fn add(&self, x: u32, y: u32) -> u32 {
        x + y
    }
}

// ------------------------------------------------------------------------------------------------
// below should be generated code
#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
enum EchoRpcEnum {
    __invalid(()),
    hello((String,)),
    add((u32, u32)),
}

trait EchoRpcClient {
    fn echo(&self, data: String) -> Result<String, Error>;
    fn add(&self, x: u32, y: u32) -> Result<u32, Error>;
}

impl EchoRpcClient for UdpClient {
    fn echo(&self, arg0: String) -> Result<String, Error> {
        let mut buf = Vec::with_capacity(1024);
        // serialize the para
        let para = EchoRpcEnum::hello((arg0,));
        encode::serialize_into(&mut buf, &para, Infinite)
            .map_err(|e| Error::ClientSerialize(e.to_string()))?;
        // call the server
        let ret = self.call_service(&buf)?;
        // deserialized the response
        encode::deserialize(&ret).map_err(|e| Error::ClientDeserialize(e.to_string()))
    }

    fn add(&self, arg0: u32, arg1: u32) -> Result<u32, Error> {
        let mut buf = Vec::with_capacity(1024);
        // serialize the para
        let para = EchoRpcEnum::add((arg0, arg1));
        encode::serialize_into(&mut buf, &para, Infinite)
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
        let req: EchoRpcEnum =
            encode::deserialize(request).map_err(|e| WireError::ServerDeserialize(e.to_string()))?;
        // dispatch call the service
        let mut buf = Vec::with_capacity(1024);
        match req {
            EchoRpcEnum::hello((arg0,)) => {
                let rsp = self.echo(arg0);
                // serialize the result
                encode::serialize_into(&mut buf, &rsp, Infinite)
                    .map_err(|e| WireError::ServerSerialize(e.to_string()))?;
            }
            EchoRpcEnum::add((arg0, arg1)) => {
                let rsp = self.add(arg0, arg1);
                // serialize the result
                encode::serialize_into(&mut buf, &rsp, Infinite)
                    .map_err(|e| WireError::ServerSerialize(e.to_string()))?;
            }
            _ => unreachable!("server dispatch unknown"),
        };
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

    for i in 0..10 {
        let data = client.add(i, i);
        println!("recv = {:?}", data);
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
