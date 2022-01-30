#[macro_use]
extern crate serde_derive;

struct Echo;

// rpc spec
trait EchoRpc: Send + Sync + 'static {
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
        use bincode as encode;
        use conetty::Client;

        let mut req = conetty::ReqBuf::new();
        // serialize the para
        let para = EchoRpcEnum::hello((arg0,));
        encode::serialize_into(&mut req, &para)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.0.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        encode::deserialize(&rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }

    pub fn add(&self, arg0: u32, arg1: u32) -> Result<u32, conetty::Error> {
        use bincode as encode;
        use conetty::Client;

        let mut req = conetty::ReqBuf::new();
        // serialize the para
        let para = EchoRpcEnum::add((arg0, arg1));
        encode::serialize_into(&mut req, &para)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.0.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        encode::deserialize(&rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }
}

struct RpcServer<T>(T);

impl<T: EchoRpc> ::std::ops::Deref for RpcServer<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: EchoRpc + ::std::panic::RefUnwindSafe> conetty::Server for RpcServer<T> {
    fn service(&self, req: &[u8], rsp: &mut conetty::RspBuf) -> Result<(), conetty::WireError> {
        use bincode as encode;

        // deserialize the request
        let req: EchoRpcEnum = encode::deserialize(req)
            .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;
        // dispatch call the service
        match req {
            EchoRpcEnum::hello((arg0,)) => {
                match ::std::panic::catch_unwind(|| self.echo(arg0)) {
                    Ok(ret) => {
                        // serialize the result
                        encode::serialize_into(rsp, &ret)
                            .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))
                    }
                    Err(_) => {
                        // panic happened inside!
                        Err(conetty::WireError::Status(
                            "rpc panicked in server!".to_owned(),
                        ))
                    }
                }
            }
            EchoRpcEnum::add((arg0, arg1)) => {
                match ::std::panic::catch_unwind(|| self.add(arg0, arg1)) {
                    Ok(ret) => {
                        // serialize the result
                        encode::serialize_into(rsp, &ret)
                            .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))
                    }
                    Err(_) => {
                        // panic happened inside!
                        Err(conetty::WireError::Status(
                            "rpc panicked in server!".to_owned(),
                        ))
                    }
                }
            }
        }
    }
}

impl<T: EchoRpc + ::std::panic::RefUnwindSafe + 'static> RpcServer<T> {
    pub fn start<L: ::std::net::ToSocketAddrs>(
        self,
        addr: L,
    ) -> ::std::io::Result<may::coroutine::JoinHandle<()>> {
        conetty::UdpServer::start(self, addr)
    }
}
// ------------------------------------------------------------------------------------------------

fn main() {
    env_logger::init();

    let addr = ("127.0.0.1", 4000);
    let server = RpcServer(Echo).start(&addr).unwrap();
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
