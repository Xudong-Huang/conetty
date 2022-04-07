#[macro_use]
extern crate token_id;
#[macro_use]
extern crate log;

use std::collections::HashMap;

use conetty::StreamExt;
use may::sync::RwLock;

pub type DispatchFn = fn(u64, &[u8], &mut conetty::RspBuf) -> Result<(), conetty::WireError>;
pub struct RpcServer {
    map: RwLock<HashMap<u64, DispatchFn>>,
}

impl conetty::Server for RpcServer {
    fn service(&self, req: &[u8], rsp: &mut conetty::RspBuf) -> Result<(), conetty::WireError> {
        use bincode as encode;

        // deserialize the request
        let req_id: u64 = encode::deserialize(req)
            .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;

        info!("req_id = {}", req_id);

        if req_id == 0 {
            let lib_path: String = encode::deserialize(&req[8..])
                .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;
            return self.register(&lib_path);
        }

        // get the dispatch_fn
        let handler = {
            let r_map = self.map.read().unwrap();
            match r_map.get(&req_id) {
                Some(f) => *f,
                None => {
                    return Err(conetty::WireError::Status(
                        "Service not available.".to_owned(),
                    ))
                }
            }
        };

        handler(req_id, req, rsp)
    }
}

impl RpcServer {
    pub fn start<L: ::std::net::ToSocketAddrs>(
        addr: L,
    ) -> ::std::io::Result<conetty::ServerInstance> {
        let server = RpcServer {
            map: RwLock::new(HashMap::new()),
        };
        conetty::TcpServer::start(server, addr)
    }

    pub fn register(&self, lib_path: &str) -> Result<(), conetty::WireError> {
        use std::path::Path;
        let lib_path = Path::new(lib_path);
        println!("lib_path = {:?}", lib_path);
        // load the map

        let map = get_dispatch_register();

        let mut w_map = self.map.write().unwrap();

        for &(req_id, f) in map {
            match w_map.insert(req_id, f) {
                Some(_) => warn!("register rpc updated. req_id={}", req_id),
                None => info!("register rpc. req_id={}", req_id),
            }
        }

        Ok(())
    }
}

pub trait RpcRegister: conetty::Client {
    // should use a path of dynamic library as input
    fn register(&self, path: &str) -> Result<(), conetty::Error> {
        use bincode as encode;

        let mut req = conetty::ReqBuf::new();
        // serialize the function id, 0 for registry
        let id = 0u64;
        encode::serialize_into(&mut req, &id)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // serialize the path
        encode::serialize_into(&mut req, path)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        encode::deserialize(rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }
}

impl<S: StreamExt> RpcRegister for conetty::MultiplexClient<S> {}
// impl RpcRegister for conetty::TcpClient {}

// rpm_impl! {
// rpc echo(data: String) -> String {
//     data
// }
//
// rpc add(x: u32, y: u32) -> u32 {
//     x + y
// }
// }

// ------------------------------------------------------------------------------------------------
// below should be generated code

// rpc impl
fn echo(data: String) -> String {
    data
}

fn add(x: u32, y: u32) -> u32 {
    x + y
}

fn dispatch_req(
    req_id: u64,
    req: &[u8],
    rsp: &mut conetty::RspBuf,
) -> Result<(), conetty::WireError> {
    use bincode as encode;
    use std::io::Cursor;

    let mut input = Cursor::new(req);

    // deserialize the request
    let _id: u64 = encode::deserialize_from(&mut input)
        .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;
    debug_assert_eq!(_id, req_id);

    // dispatch call the service
    if req_id == token_id!(echo) {
        // 0 => {
        let (data,): (String,) = encode::deserialize_from(&mut input)
            .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;
        let ret = echo(data);
        // serialize the result
        encode::serialize_into(rsp, &ret)
            .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))
    } else if req_id == token_id!(add) {
        let (x, y): (u32, u32) = encode::deserialize_from(&mut input)
            .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;
        let ret = add(x, y);
        // serialize the result
        encode::serialize_into(rsp, &ret)
            .map_err(|e| conetty::WireError::ServerSerialize(e.to_string()))
    } else {
        unreachable!("unknown req_id = {}", req_id);
    }
}

pub fn get_dispatch_register() -> &'static [(u64, DispatchFn)] {
    const HANDLER_MAP: &[(u64, DispatchFn)] = &[
        (token_id!(echo), dispatch_req),
        (token_id!(add), dispatch_req),
    ];
    HANDLER_MAP
}

pub trait RpcClient: conetty::Client {
    fn echo(&mut self, arg0: String) -> Result<String, conetty::Error> {
        use bincode as encode;

        let mut req = conetty::ReqBuf::new();
        // serialize the function id
        let id = token_id!(echo);
        encode::serialize_into(&mut req, &id)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // serialize the para
        let para = (arg0,);
        encode::serialize_into(&mut req, &para)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        encode::deserialize(rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }

    fn add(&mut self, arg0: u32, arg1: u32) -> Result<u32, conetty::Error> {
        use bincode as encode;

        let mut req = conetty::ReqBuf::new();
        // serialize the function id
        let id = token_id!(add);
        encode::serialize_into(&mut req, &id)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // serialize the para
        let para = (arg0, arg1);
        encode::serialize_into(&mut req, &para)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        encode::deserialize(rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }
}

impl<S: StreamExt> RpcClient for conetty::MultiplexClient<S> {}
// impl RpcClient for conetty::TcpClient {}

// ------------------------------------------------------------------------------------------------

fn main() {
    use conetty::MultiplexClient;
    env_logger::init();

    let addr = ("127.0.0.1", 4000);
    let _server = RpcServer::start(&addr).unwrap();

    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let mut client = MultiplexClient::new(tcp_stream).unwrap();
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

    client.register("file_path").unwrap();

    for i in 0..10 {
        let s = format!("Hello World! id={}", i);
        let data = client.echo(s);
        println!("recv = {:?}", data);
    }

    for i in 0..100 {
        let data = client.add(i, i);
        println!("recv = {:?}", data);
    }
}
