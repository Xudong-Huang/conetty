extern crate conetty;
extern crate coroutine;
extern crate env_logger;

use std::str;
use std::error::Error as StdError;
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
trait EchoRpcClient {
    fn echo(&self, data: &str) -> Result<String, Error>;
}

impl EchoRpcClient for UdpClient {
    fn echo(&self, data: &str) -> Result<String, Error> {
        let ret = self.call_service(data.as_bytes())?;
        String::from_utf8(ret).map_err(|e| Error::ClientDeserialize(e.description().into()))
    }
}

impl Service for Echo {
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, WireError> {
        // do some parsing here
        let s = str::from_utf8(request)
            .map_err(|e| WireError::ServerDeserialize(e.description().into()))?;
        // serialize the result
        let ret = self.echo(s.into());
        Ok(ret.into_bytes())
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
        let data = client.echo(&s);
        println!("recv = {:?}", data);
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
