use std::io::{self, Cursor, Read, Write, ErrorKind};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use {Error, WireError};

// max frame len
const FRAME_MAX_LEN: u64 = 1024 * 1024;

/// raw frame wrapper, low level protocol
#[derive(Debug)]
pub struct Frame {
    /// frame id, req and rsp has the same id
    pub id: u64,
    /// payload data
    pub data: Vec<u8>,
}

impl Frame {
    /// decode a frame from the reader
    pub fn decode_from<R: Read>(r: &mut R) -> io::Result<Self> {
        let id = r.read_u64::<BigEndian>()?;
        info!("decode id = {:?}", id);

        let len = r.read_u64::<BigEndian>()?;
        info!("decode len = {:?}", len);

        if len > FRAME_MAX_LEN {
            let s = format!("decode too big frame length. len={}", len);
            error!("{}", s);
            return Err(io::Error::new(ErrorKind::InvalidInput, s));
        }

        let mut data = Vec::<u8>::new();
        data.resize(len as usize, 0);
        r.read_exact(&mut data)?;
        Ok(Frame {
            id: id,
            data: data,
        })
    }

    /// decode a response from the frame, this would return the rsp raw bufer
    /// you need to deserialized from it into the real type
    pub fn decode_rsp(&self) -> Result<&[u8], Error> {
        use Error::*;

        let mut r = Cursor::new(&self.data);

        let ty = r.read_u8()?;
        // we don't need to check len here, frame is checked already
        let len = r.read_u64::<BigEndian>()? as usize;

        let buf = r.into_inner();
        let data = &buf[9..len + 9];

        // info!("decode response, ty={}, len={}", ty, len);
        match ty {
            0 => Ok(data),
            1 => Err(ServerDeserialize(unsafe { String::from_utf8_unchecked(data.into()) })),
            2 => Err(ServerSerialize(unsafe { String::from_utf8_unchecked(data.into()) })),
            _ => {
                let s = format!("invalid response type. ty={}", ty);
                error!("{}", s);
                Err(ClientDeserialize(s))
            }
        }
    }
}

/// raw frame buffer that can be serialized into
pub struct FrameBuf(Cursor<Vec<u8>>);

impl FrameBuf {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(1024);
        buf.resize(16, 0);
        let mut cursor = Cursor::new(buf);
        // leave enough space to write id and len
        cursor.set_position(16);
        FrameBuf(cursor)
    }

    /// convert self into raw buf that can be send as a frame
    pub fn finish(self, id: u64) -> Vec<u8> {
        let mut cursor = self.0;
        let len = cursor.get_ref().len() as u64;
        assert!(len <= FRAME_MAX_LEN);

        // write from start
        cursor.set_position(0);
        cursor.write_u64::<BigEndian>(id).unwrap();
        info!("encode id = {:?}", id);

        // adjust the data length
        cursor.write_u64::<BigEndian>(len - 16).unwrap();
        info!("encode len = {:?}", len);

        cursor.into_inner()
    }
}

impl Write for FrameBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// rsp frame buffer that can be serialized into
pub struct RspBuf(Cursor<Vec<u8>>);

impl RspBuf {
    pub fn new() -> Self {
        let mut buf = Vec::with_capacity(512);
        // id + len + ty + len + data
        buf.resize(25, 0);
        let mut cursor = Cursor::new(buf);
        // leave enough space to write id and len
        cursor.set_position(25);
        RspBuf(cursor)
    }

    /// convert self into raw buf that can be send as a frame
    pub fn finish(self, id: u64, ret: Result<(), WireError>) -> Vec<u8> {
        let mut cursor = self.0;
        let dummy = vec![0; 0];

        let (ty, len, data) = match ret {
            Ok(_) => (0, cursor.get_ref().len() - 25, dummy.as_slice()),
            Err(ref e) => {
                match *e {
                    WireError::ServerDeserialize(ref s) => (1, s.len(), s.as_bytes()),
                    WireError::ServerSerialize(ref s) => (2, s.len(), s.as_bytes()),
                }
            }
        };

        let len = len as u64;
        assert!(len < FRAME_MAX_LEN);

        // write from start
        cursor.set_position(0);
        cursor.write_u64::<BigEndian>(id).unwrap();
        info!("encode id = {:?}", id);

        // adjust the data length
        cursor.write_u64::<BigEndian>(len + 9).unwrap();
        info!("encode len = {:?}", len);

        // write the type
        cursor.write_u8(ty).unwrap();
        // write the len
        cursor.write_u64::<BigEndian>(len).unwrap();
        // write the data into the writer
        if ty != 0 {
            cursor.get_mut().resize(len as usize + 25, 0);
            cursor.write_all(data).unwrap();
        }

        cursor.into_inner()
    }
}

impl Write for RspBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}