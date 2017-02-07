use std::io::{self, Read, Write};
use errors::{Error, WireError};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

// decode response
pub fn decode_from<R: Read>(r: &mut R) -> Result<Vec<u8>, Error> {
    use Error::*;

    let ty = r.read_u8()?;
    // we don't need to check len here, frame is checked already
    let len = r.read_u64::<BigEndian>()? as usize;
    let mut data = vec![0; len];
    r.read_exact(&mut data)?;

    // info!("decode response, ty={}, len={}", ty, len);
    match ty {
        0 => Ok(data),
        1 => Err(ServerDeserialize(unsafe { String::from_utf8_unchecked(data) })),
        2 => Err(ServerSerialize(unsafe { String::from_utf8_unchecked(data) })),
        _ => {
            let s = format!("invalid response type. ty={}", ty);
            error!("{}", s);
            Err(ClientDeserialize(s))
        }
    }
}

// encode response
#[allow(dead_code)]
pub fn encode_into<W: Write>(w: &mut W, rsp: &Result<Vec<u8>, WireError>) -> io::Result<()> {
    let (ty, len, data) = match *rsp {
        Ok(ref d) => (0, d.len(), d.as_slice()),
        Err(ref e) => {
            match *e {
                WireError::ServerDeserialize(ref s) => (1, s.len(), s.as_bytes()),
                WireError::ServerSerialize(ref s) => (2, s.len(), s.as_bytes()),
            }
        }
    };

    // write the type into the writer
    w.write_u8(ty)?;
    // write the len into the writer
    w.write_u64::<BigEndian>(len as u64)?;
    // write the data into the writer
    w.write_all(data)
}
