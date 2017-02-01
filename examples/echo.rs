extern crate conetty;
extern crate env_logger;
#[macro_use]
extern crate serde_derive;


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
    hello((String,)),
    add((u32, u32)),
}

pub struct EchoRpcClient(conetty::UdpClient);

impl EchoRpcClient {
    pub fn connect<L: ::std::net::ToSocketAddrs>(addr: L) -> ::std::io::Result<EchoRpcClient> {
        conetty::UdpClient::connect(addr).map(EchoRpcClient)
    }

    pub fn set_timeout(&mut self, timeout: ::std::time::Duration) {
        self.0.set_timeout(timeout)
    }

    pub fn echo(&self, arg0: String) -> Result<String, conetty::Error> {
        use conetty::Client;
        use conetty::bincode::serde as encode;
        use conetty::bincode::SizeLimit::Infinite;

        let mut buf = Vec::with_capacity(1024);
        // serialize the para
        let para = EchoRpcEnum::hello((arg0,));
        encode::serialize_into(&mut buf, &para, Infinite)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let ret = self.0.call_service(&buf)?;
        // deserialized the response
        encode::deserialize(&ret).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }

    pub fn add(&self, arg0: u32, arg1: u32) -> Result<u32, conetty::Error> {
        use conetty::Client;
        use conetty::bincode::serde as encode;
        use conetty::bincode::SizeLimit::Infinite;

        let mut buf = Vec::with_capacity(1024);
        // serialize the para
        let para = EchoRpcEnum::add((arg0, arg1));
        encode::serialize_into(&mut buf, &para, Infinite)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let ret = self.0.call_service(&buf)?;
        // deserialized the response
        encode::deserialize(&ret).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }
}

impl conetty::Server for Echo {
    fn service(&self, request: &[u8]) -> Result<Vec<u8>, conetty::WireError> {
        use conetty::bincode::serde as encode;
        use conetty::bincode::SizeLimit::Infinite;

        // deserialize the request
        let req: EchoRpcEnum = encode::deserialize(request)
            .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;
        // dispatch call the service
        let mut buf = Vec::with_capacity(512);
        match req {
            EchoRpcEnum::hello((arg0,)) => {
                let rsp = self.echo(arg0);
                // serialize the result
                encode::serialize_into(&mut buf, &rsp, Infinite)
                    .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))?;
            }
            EchoRpcEnum::add((arg0, arg1)) => {
                let rsp = self.add(arg0, arg1);
                // serialize the result
                encode::serialize_into(&mut buf, &rsp, Infinite)
                    .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))?;
            }
        };
        // send the response
        Ok(buf)
    }
}

impl Echo {
    pub fn start<L: ::std::net::ToSocketAddrs>
        (self,
         addr: L)
         -> ::std::io::Result<conetty::coroutine::JoinHandle<()>> {
        conetty::UdpServer::start(self, addr)
    }
}
// ------------------------------------------------------------------------------------------------

fn main() {
    env_logger::init().unwrap();

    let addr = ("127.0.0.1", 4000);
    let server = Echo.start(&addr).unwrap();
    let mut client = EchoRpcClient::connect(addr).unwrap();
    client.set_timeout(::std::time::Duration::from_millis(100));

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
