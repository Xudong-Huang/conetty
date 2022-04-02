pub trait StreamExt: Sized + Send + 'static {
    fn try_clone(&self) -> std::io::Result<Self>;
    fn set_read_timeout(&mut self, timeout: std::time::Duration) -> std::io::Result<()>;
}

macro_rules! impl_stream_ext {
    ($name: ty) => {
        impl StreamExt for $name {
            fn try_clone(&self) -> std::io::Result<Self> {
                (*self).try_clone()
            }
            fn set_read_timeout(&mut self, timeout: std::time::Duration) -> std::io::Result<()> {
                (*self).set_read_timeout(Some(timeout))
            }
        }
    };
}

impl_stream_ext!(std::net::TcpStream);
impl_stream_ext!(may::net::TcpStream);
#[cfg(unix)]
impl_stream_ext!(std::os::unix::net::UnixStream);
#[cfg(unix)]
impl_stream_ext!(may::os::unix::net::UnixStream);
