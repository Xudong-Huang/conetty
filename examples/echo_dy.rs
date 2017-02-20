#![feature(associated_consts)]

extern crate conetty;
extern crate bincode;
extern crate env_logger;
#[macro_use]
extern crate tocken_id;
#[macro_use]
extern crate log;


// rpm_impl! {
// rpc echo(data: String) -> String {
//     data
// }
//
// rpc add(x: u32, y: u32) -> u32 {
//     x + y
// }
// }

// rpc spec
trait EchoRpc: Send + Sync + 'static {
    fn echo(&self, data: String) -> String;
    fn add(&self, x: u32, y: u32) -> u32;
}

// server implementation
// impl EchoRpc for Echo {
//     fn echo(data: String) -> String {
//         data
//     }
//
//     fn add(x: u32, y: u32) -> u32 {
//         x + y
//     }
// }

// ------------------------------------------------------------------------------------------------
// below should be generated code

pub trait RpcClient: conetty::Client {
    fn echo(&self, arg0: String) -> Result<String, conetty::Error> {
        use bincode as encode;
        use bincode::SizeLimit::Infinite;

        let mut req = conetty::ReqBuf::new();
        // serialize the function id
        let id = tocken_id!(echo);
        encode::serialize_into(&mut req, &id, Infinite)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // serialize the para
        let para = (arg0,);
        encode::serialize_into(&mut req, &para, Infinite)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        encode::deserialize(&rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }

    fn add(&self, arg0: u32, arg1: u32) -> Result<u32, conetty::Error> {
        use bincode as encode;
        use bincode::SizeLimit::Infinite;

        let mut req = conetty::ReqBuf::new();
        // serialize the function id
        let id = tocken_id!(add);
        encode::serialize_into(&mut req, &id, Infinite)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // serialize the para
        let para = (arg0, arg1);
        encode::serialize_into(&mut req, &para, Infinite)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        encode::deserialize(&rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }
}

impl RpcClient for conetty::MultiplexClient {}
impl RpcClient for conetty::TcpClient {}


use std::collections::HashMap;
use conetty::coroutine::sync::RwLock;
pub type DispatchFn = &'static fn(u64, &[u8], &mut conetty::RspBuf) -> Result<(), conetty::WireError>;
pub struct RpcServer {
    map: RwLock<HashMap<u64, DispatchFn>>,
}

impl conetty::Server for RpcServer {
    fn service(&self, req: &[u8], rsp: &mut conetty::RspBuf) -> Result<(), conetty::WireError> {
        use bincode as encode;
        use bincode::SizeLimit::Infinite;

        // deserialize the request
        let req_id: u64 = encode::deserialize(req)
            .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;

        info!("req_id = {}", req_id);
        // get the dispatch_fn
        let f = {
            let r_map = self.map.read().unwrap();
            match r_map.get(&req_id) {
                Some(f) => *f,
                None => return Err(conetty::WireError::Status(404)),
            }
        };

        f(req_id, req, rsp)
        // // dispatch call the service
        // match req {
        //     EchoRpcEnum::hello((arg0,)) => {
        //         let ret = self.echo(arg0);
        //         // serialize the result
        //         encode::serialize_into(rsp, &ret, Infinite)
        //             .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))
        //     }
        //     EchoRpcEnum::add((arg0, arg1)) => {
        //         let ret = self.add(arg0, arg1);
        //         // serialize the result
        //         encode::serialize_into(rsp, &ret, Infinite)
        //             .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))
        //     }
        // }
    }
}

impl RpcServer {
    pub fn start<L: ::std::net::ToSocketAddrs>
        (addr: L)
         -> ::std::io::Result<conetty::coroutine::JoinHandle<()>> {
        let server = RpcServer { map: RwLock::new(HashMap::new()) };
        conetty::TcpServer::start(server, addr)
    }
}
// ------------------------------------------------------------------------------------------------

fn main() {
    use conetty::MultiplexClient;
    env_logger::init().unwrap();

    let addr = ("127.0.0.1", 4000);
    let server = RpcServer::start(&addr).unwrap();
    let mut client = MultiplexClient::connect(addr).unwrap();
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
