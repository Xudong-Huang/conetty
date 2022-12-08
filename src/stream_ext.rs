use std::io::{self, Read, Write};
use std::time::Duration;

use may::io::SplitIo;

pub trait StreamExt: Sized + SplitIo + Read + Write + Send + 'static {
    fn try_clone(&self) -> io::Result<Self>;
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()>;
}

macro_rules! impl_stream_ext {
    ($name: ty) => {
        impl StreamExt for $name {
            fn try_clone(&self) -> io::Result<Self> {
                (*self).try_clone()
            }
            fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()> {
                (*self).set_read_timeout(Some(timeout))
            }
        }
    };
}

impl_stream_ext!(may::net::TcpStream);
#[cfg(unix)]
impl_stream_ext!(may::os::unix::net::UnixStream);
