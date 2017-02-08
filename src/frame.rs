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

    /// encode one frame into the writer
    pub fn encode_into<W: Write>(w: &mut W, id: u64, data: &[u8]) -> io::Result<()> {
        let len = data.len() as u64;
        if len > FRAME_MAX_LEN {
            let s = format!("encode too big frame length. len={}", len);
            error!("{}", s);
            return Err(io::Error::new(ErrorKind::InvalidInput, s));
        }

        w.write_u64::<BigEndian>(id)?;
        info!("encode id = {:?}", id);

        w.write_u64::<BigEndian>(len)?;
        info!("encode len = {:?}", len);

        w.write_all(data)?;
        w.flush()
    }

    /// encode one rsp to the writer
    pub fn encode_rsp<W: Write>(w: &mut W,
                                id: u64,
                                rsp: Result<Vec<u8>, WireError>)
                                -> io::Result<()> {
        let (ty, len, data) = match rsp {
            Ok(ref d) => (0, d.len(), d.as_slice()),
            Err(ref e) => {
                match *e {
                    WireError::ServerDeserialize(ref s) => (1, s.len(), s.as_bytes()),
                    WireError::ServerSerialize(ref s) => (2, s.len(), s.as_bytes()),
                }
            }
        };

        // adjust len = len(data) + 1(ty) + 8(len)
        let len1 = len as u64 + 9;

        if len1 > FRAME_MAX_LEN {
            let s = format!("encode too big frame length. len={}", len);
            error!("{}", s);
            return Err(io::Error::new(ErrorKind::InvalidInput, s));
        }

        // write the id
        w.write_u64::<BigEndian>(id)?;
        info!("encode id = {:?}", id);

        // write the length
        w.write_u64::<BigEndian>(len1)?;
        info!("encode len = {:?}", len);

        // write the type into the writer
        w.write_u8(ty)?;
        // write the len into the writer
        w.write_u64::<BigEndian>(len as u64)?;
        // write the data into the writer
        w.write_all(data)?;
        w.flush()
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
