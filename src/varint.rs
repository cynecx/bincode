pub mod le_prefix {
    use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
    use std::io::{Read, Write};

    use crate::Error;

    pub fn size_of(val: u64) -> usize {
        let bits = 64 - (val | 0x01).leading_zeros() as usize;

        if bits > 56 {
            9
        } else {
            1 + (bits - 1) / 7
        }
    }

    pub fn encode<W: Write>(writer: &mut W, val: u64) -> Result<(), Error> {
        let bits = 64 - (val | 0x01).leading_zeros();

        if bits <= 56 {
            let num_bytes = 1 + (bits - 1) / 7;
            let val = ((val << 1) | 1) << (num_bytes - 1);

            let mut buf = [0u8; 8];
            LittleEndian::write_u64(&mut buf, val);
            writer.write_all(unsafe {
                // &buf[..num_bytes as usize]
                // This is safe because `1 + (bits - 1) / 7` (bits <= 56) produces [0, 8) values
                std::slice::from_raw_parts(buf.as_ptr(), num_bytes as usize)
            })?;
        } else {
            let mut buf = [0u8; 9];
            LittleEndian::write_u64(&mut buf[1..], val);
            writer.write_all(&buf)?;
        }

        Ok(())
    }

    pub fn decode<R: Read>(reader: &mut R) -> Result<u64, Error> {
        let prefix_byte = reader.read_u8()?;
        let num_bytes = 1 + (prefix_byte as u32 | 0x100).trailing_zeros();

        Ok(match num_bytes {
            1 => prefix_byte as u64 >> 1,
            2...8 => {
                let mut buf = [0u8; 8];
                reader.read_exact(unsafe {
                    // &mut buf[..(num_bytes as usize - 1)]
                    // This is safe because of num_bytes <= 8
                    std::slice::from_raw_parts_mut(buf.as_mut_ptr(), num_bytes as usize - 1)
                })?;

                let val = LittleEndian::read_u64(&buf) << (8 - num_bytes);
                let prefix_part = (prefix_byte as u64) >> num_bytes;

                val | prefix_part
            }
            _ => reader.read_u64::<LittleEndian>()?,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::{Seek, SeekFrom};

        fn test_num(val: u64) {
            let mut buf = [0u8; 10];
            let mut cursor = std::io::Cursor::new(&mut buf[..]);

            encode(&mut cursor, val).unwrap();

            let len = cursor.seek(SeekFrom::Current(0)).unwrap();
            assert_eq!(len as usize, size_of(val));

            cursor.seek(SeekFrom::Start(0)).unwrap();
            assert_eq!(val, decode(&mut cursor).unwrap());
        }

        #[test]
        fn test_encode_decode() {
            for i in 0..64 {
                let base = 1u64 << i;
                test_num(base);
                test_num(base - 1);
            }
        }

    }
}

pub use self::le_prefix::*;
