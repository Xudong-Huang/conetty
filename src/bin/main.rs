#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate coroutine;

use std::ops::{Deref, DerefMut};
use std::io::{self, Read, Write};
use coroutine::net::UdpSocket;
use bincode::{serde, SizeLimit};

struct Udp(UdpSocket);

impl Deref for Udp {
    type Target = UdpSocket;
    fn deref(&self) -> &UdpSocket {
        &self.0
    }
}

impl DerefMut for Udp {
    fn deref_mut(&mut self) -> &mut UdpSocket {
        &mut self.0
    }
}

impl Read for Udp {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }
}

impl Write for Udp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let server_addr = ("127.0.0.1", 1111);
    let client_addr = ("127.0.0.1", 2222);
    let server_sock = UdpSocket::bind(&server_addr).unwrap();
    let client_sock = UdpSocket::bind(&client_addr).unwrap();
    client_sock.connect(&server_addr).unwrap();
    server_sock.connect(&client_addr).unwrap();
    let mut client = Udp(client_sock);
    let mut server = Udp(server_sock);
    let point = Point { x: 1, y: 2 };

    // send the point from client to server_addr
    serde::serialize_into(&mut client, &point, SizeLimit::Infinite).unwrap();

    // recv the point inserver
    let mut p: Point = serde::deserialize_from(&mut server, SizeLimit::Infinite).unwrap();
    println!("recv point = {:?}", p);
    p.x = 100;

    serde::serialize_into(&mut server, &p, SizeLimit::Infinite).unwrap();
    let p1: Point = serde::deserialize_from(&mut client, SizeLimit::Infinite).unwrap();
    println!("recv point = {:?}", p1);
}
