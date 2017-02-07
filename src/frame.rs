use std::io::{Result, Read, Write, Error, ErrorKind};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

// max frame len
const FRAME_MAX_LEN: u64 = 1024 * 1024;

/// raw frame wrapper
pub struct Frame {
    pub id: u64,
    pub data: Vec<u8>,
}

impl Frame {
    /// decode a frame from the reader
    pub fn decode_from<R: Read>(r: &mut R) -> Result<Self> {
        let id = r.read_u64::<BigEndian>()?;
        info!("decode id = {:?}", id);

        let len = r.read_u64::<BigEndian>()?;
        info!("decode len = {:?}", len);

        if len > FRAME_MAX_LEN {
            let s = format!("too big frame length. len={}", len);
            error!("{}", s);
            return Err(Error::new(ErrorKind::InvalidInput, s));
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
    pub fn encode_into<W: Write>(w: &mut W, id: u64, data: &[u8]) -> Result<()> {
        w.write_u64::<BigEndian>(id)?;
        info!("encode id = {:?}", id);

        let len = data.len() as u64;
        w.write_u64::<BigEndian>(len)?;
        info!("encode len = {:?}", len);

        w.write_all(data)?;
        w.flush()
    }
}
