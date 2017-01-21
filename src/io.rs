use std::io::{self, Read, Write};
use std::net::ToSocketAddrs;
use coroutine::sync::Mutex;


pub struct RpcIo<Io: Read + Write> {
    // used to sequence the write action
    mutex: Mutex<()>,
    io: Io,
}

impl<Io: Read + Write> RpcIo<Io> {
    pub fn new(io: Io) -> Self {
        RpcIo {
            mutex: Mutex::new(()),
            io: io,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read(buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self.mutex.lock().unwrap();
        self.io.write(buf)
    }
}
