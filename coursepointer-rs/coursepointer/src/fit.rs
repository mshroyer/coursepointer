use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FitEncodeError {
    #[error("writing to output")]
    Write(#[from] std::io::Error),
    #[error("encoding integer")]
    IntegerEncoding(#[from] std::num::TryFromIntError),
}

type Result<T> = std::result::Result<T, FitEncodeError>;

trait Encode {
    fn encode<W: Write>(&self, w: &mut W) -> Result<()>;

    fn size(&self) -> usize;
}

/// Implements the Garmin FIT CRC algorithm.
///
/// A direct transcription of Garmin's reference implementation at
/// https://developer.garmin.com/fit/protocol/
struct Crc {
    sum: u16,
}

static CRC_TABLE: &'static [u16] = &[
    0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800, 0xB401,
    0x5000, 0x9C01, 0x8801, 0x4400,
];

impl Crc {
    pub fn new() -> Self {
        // Garmin's docs don't say so explicitly, but the starting value is zero.
        Self { sum: 0 }
    }

    pub fn add_byte(&mut self, byte: u8) {
        // Checksum lower four bits
        let mut tmp = CRC_TABLE[(self.sum & 0x0F) as usize];
        self.sum = (self.sum >> 4) & 0x0FFF;
        self.sum = self.sum ^ tmp ^ CRC_TABLE[(byte & 0x0F) as usize];

        // Checksum upper four bits
        tmp = CRC_TABLE[(self.sum & 0x0F) as usize];
        self.sum = (self.sum >> 4) & 0x0FFF;
        self.sum = self.sum ^ tmp ^ CRC_TABLE[(byte >> 4) as usize];
    }

    pub fn add_bytes(&mut self, byte: &[u8]) {
        for byte in byte {
            self.add_byte(*byte);
        }
    }
}

impl Encode for Crc {
    fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u16::<LittleEndian>(self.sum)?;
        Ok(())
    }

    fn size(&self) -> usize {
        2
    }
}

struct CheckSummingWriter<'a, W: Write> {
    crc: Crc,
    base: &'a mut W,
}

impl<'a, W: Write> CheckSummingWriter<'a, W> {
    fn new(base: &'a mut W) -> Self {
        Self {
            crc: Crc::new(),
            base,
        }
    }

    /// Finish using the writer and write the CRC to the end of the stream.
    fn finish(self) -> Result<()> {
        self.base.write_u16::<LittleEndian>(self.crc.sum)?;
        Ok(())
    }
}

impl<W: Write> Write for CheckSummingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.crc.add_bytes(buf);
        self.base.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.base.flush()
    }
}

enum ProtocolVersion {
    V10,
}

struct FileHeader {
    protocol_version: ProtocolVersion,
    profile_version: u16,
    data_size: u32,
}

impl FileHeader {
    pub fn new(profile_version: u16, data_size: usize) -> Result<Self> {
        let data_size_u32 = u32::try_from(data_size)?;
        Ok(Self {
            protocol_version: ProtocolVersion::V10,
            profile_version,
            data_size: data_size_u32,
        })
    }
}

impl Encode for FileHeader {
    fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u8(14)?;
        w.write_u8(match self.protocol_version {
            ProtocolVersion::V10 => 0x10u8,
        })?;
        w.write_u16::<LittleEndian>(self.profile_version)?;
        w.write_u32::<LittleEndian>(self.data_size)?;
        write!(w, ".FIT")?;
        Ok(())
    }

    fn size(&self) -> usize {
        14
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckSummingWriter, Crc, Encode, FileHeader, Result};

    #[test]
    fn test_header_crc() {
        let mut crc = Crc::new();
        // A header from a FIT file I exported from Garmin Connect, minus its CRC bytes.
        crc.add_bytes(&[
            0x0e, 0x10, 0xb2, 0x52, 0x88, 0x42, 0x00, 0x00, 0x2e, 0x46, 0x49, 0x54,
        ]);
        // The CRC value from the last two bytes of the header, interpreted as little endian.
        assert_eq!(crc.sum, 0xf94b);
    }

    #[test]
    fn test_header_encode() -> Result<()> {
        let mut buf : Vec<u8> = vec![];
        let mut writer = CheckSummingWriter::new(&mut buf);
        let header = FileHeader::new(21170u16, 17032usize)?;
        header.encode(&mut writer)?;
        writer.finish()?;

        assert_eq!(buf, &[
            0x0e, 0x10, 0xb2, 0x52, 0x88, 0x42, 0x00, 0x00, 0x2e, 0x46, 0x49, 0x54, 0x4b, 0xf9
        ]);

        Ok(())
    }
}
