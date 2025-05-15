use std::io::Write;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FitEncodeError {
    #[error("writing to output")]
    Write(#[from] std::io::Error),
    #[error("encoding integer")]
    IntegerEncoding(#[from] std::num::TryFromIntError),
    #[error("encoding string")]
    StringEncoding,
}

type Result<T> = std::result::Result<T, FitEncodeError>;

fn write_string_field<W: Write>(s: &str, field_size: usize, w: &mut W) -> Result<()> {
    if s.len() >= field_size - 1 {
        return Err(FitEncodeError::StringEncoding);
    }
    w.write_all(s.as_bytes())?;
    for _ in 0..(field_size - s.len()) {
        w.write_u8(0)?;
    }
    Ok(())
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
    fn new() -> Self {
        // Garmin's docs don't say so explicitly, but the starting value is zero.
        Self { sum: 0 }
    }

    fn add_byte(&mut self, byte: u8) {
        // Checksum lower four bits
        let mut tmp = CRC_TABLE[(self.sum & 0x0F) as usize];
        self.sum = (self.sum >> 4) & 0x0FFF;
        self.sum = self.sum ^ tmp ^ CRC_TABLE[(byte & 0x0F) as usize];

        // Checksum upper four bits
        tmp = CRC_TABLE[(self.sum & 0x0F) as usize];
        self.sum = (self.sum >> 4) & 0x0FFF;
        self.sum = self.sum ^ tmp ^ CRC_TABLE[(byte >> 4) as usize];
    }

    fn add_bytes(&mut self, byte: &[u8]) {
        for byte in byte {
            self.add_byte(*byte);
        }
    }
}

/// A Write implementation that wraps another Write and computes a checksum over data written.
struct CheckSummingWrite<'a, W: Write> {
    crc: Crc,
    base: &'a mut W,
}

impl<'a, W: Write> CheckSummingWrite<'a, W> {
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

impl<W: Write> Write for CheckSummingWrite<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.crc.add_bytes(buf);
        self.base.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.base.flush()
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum ProtocolVersion {
    V10 = 0x10,
}

pub struct FileHeader {
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

    pub fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u8(14)?;
        w.write_u8(self.protocol_version as u8)?;
        w.write_u16::<LittleEndian>(self.profile_version)?;
        w.write_u32::<LittleEndian>(self.data_size)?;
        write!(w, ".FIT")?;
        Ok(())
    }
}

struct FieldDefinition {
    field_number: u8,
    size: u8,
    base_type: u8,
}

impl FieldDefinition {
    fn new(field_number: u8, size: u8, base_type: u8) -> Self {
        Self {
            field_number,
            size,
            base_type,
        }
    }

    fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u8(self.field_number)?;
        w.write_u8(self.size)?;
        w.write_u8(self.base_type)?;
        Ok(())
    }
}

#[repr(u16)]
#[derive(Clone, Copy, Debug)]
enum GlobalMessage {
    FileId = 0u16,
    Course = 31u16,
}

pub struct DefinitionFrame {
    global_message: GlobalMessage,
    local_message_type: u8,
    field_definitions: Vec<FieldDefinition>,
}

impl DefinitionFrame {
    fn new(
        global_message: GlobalMessage,
        local_message_type: u8,
        field_definitions: Vec<FieldDefinition>,
    ) -> Self {
        Self {
            global_message,
            local_message_type,
            field_definitions,
        }
    }

    fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u8(0b01000000 | (self.local_message_type & 0b00001111))?;
        w.write_u8(0x00)?; // reserved
        w.write_u8(0x01)?; // architecture = big endian
        w.write_u16::<BigEndian>(self.global_message as u16)?;
        w.write_u8(u8::try_from(self.field_definitions.len())?)?;

        for def in &self.field_definitions {
            def.encode(w)?;
        }
        Ok(())
    }
}

struct RecordMessage {
    lat: i32,
    lon: i32,
    dist_m: i32,
    time_s: u32,
}

impl RecordMessage {
    fn message_id() -> u16 {
        20
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(0, 4, 133),   // lat
            FieldDefinition::new(1, 4, 133),   // lon
            FieldDefinition::new(5, 4, 133),   // distance
            FieldDefinition::new(253, 4, 134), // timestamp
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_i32::<BigEndian>(self.lat)?;
        w.write_i32::<BigEndian>(self.lon)?;
        w.write_i32::<BigEndian>(self.dist_m)?;
        w.write_u32::<BigEndian>(self.time_s)?;
        Ok(())
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum Sport {
    Cycling = 2u8,
}

struct CourseMessage {
    name: String,
    sport: Sport,
}

impl CourseMessage {
    fn new(name: String, sport: Sport) -> Self {
        Self { name, sport }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(5, 32, 7), // name
            FieldDefinition::new(4, 1, 0),  // sport
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        write_string_field(self.name.as_str(), 32, w)?;
        w.write_u8(self.sport as u8)?;
        Ok(())
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum FileType {
    Course = 6,
}

struct FileIdMessage {
    file_type: FileType,
}

impl FileIdMessage {
    fn new(file_type: FileType) -> Self {
        Self { file_type }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![FieldDefinition::new(0, 1, 0)]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u8(self.file_type as u8)?;
        Ok(())
    }
}

struct LapMessage {}

pub struct CourseFile {
    profile_version: u16,
    name: String,
    records: Vec<RecordMessage>,
}

impl CourseFile {
    pub fn new(profile_version: u16, name: String) -> Self {
        Self {
            profile_version,
            name,
            records: vec![],
        }
    }

    pub fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        // File header
        let mut hw = CheckSummingWrite::new(w);
        let h = FileHeader::new(0u16, self.get_data_size())?;
        h.encode(&mut hw)?;
        hw.finish()?;

        // File data
        let mut dw = CheckSummingWrite::new(w);

        DefinitionFrame::new(
            GlobalMessage::FileId,
            0u8,
            FileIdMessage::field_definitions(),
        )
        .encode(&mut dw)?;
        FileIdMessage::new(FileType::Course).encode(0u8, &mut dw)?;

        DefinitionFrame::new(
            GlobalMessage::Course,
            1u8,
            CourseMessage::field_definitions(),
        )
        .encode(&mut dw)?;
        CourseMessage::new(self.name.clone(), Sport::Cycling).encode(1u8, &mut dw)?;

        dw.finish()?;

        Ok(())
    }

    /// Computes the total size of the data segment of this file, including definition messages
    /// and data messages.
    fn get_data_size(&self) -> usize {
        let mut sz = 0usize;

        sz += CourseFile::get_definition_message_size(FileIdMessage::field_definitions().len());
        sz += CourseFile::get_data_message_size(FileIdMessage::field_definitions());

        sz += CourseFile::get_definition_message_size(CourseMessage::field_definitions().len());
        sz += CourseFile::get_data_message_size(CourseMessage::field_definitions());

        // sz += CourseFile::get_definition_message_size(LapMessage::field_definitions().len());
        // sz += CourseFile::get_data_message_size(LapMessage::field_definitions());

        // sz += CourseFile::get_definition_message_size(RecordMessage::field_definitions().len());
        // sz += self.records.len()
        //     * CourseFile::get_data_message_size(RecordMessage::field_definitions());

        sz
    }

    /// Computes the size of a definition message based on the number of field definitions, assuming
    /// no developer data fields.
    fn get_definition_message_size(num_defs: usize) -> usize {
        6usize + 3 * num_defs
    }

    /// Computes the size of a single instance of a data message, given its field definitions.
    fn get_data_message_size<I>(defs: I) -> usize
    where
        I: IntoIterator<Item = FieldDefinition>,
    {
        1usize + defs.into_iter().map(|def| def.size as usize).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::{Crc, FileHeader, Result};

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
        let mut buf: Vec<u8> = vec![];
        let header = FileHeader::new(21170u16, 17032usize)?;
        header.encode(&mut buf)?;

        assert_eq!(
            buf,
            &[
                0x0e, 0x10, 0xb2, 0x52, 0x88, 0x42, 0x00, 0x00, 0x2e, 0x46, 0x49, 0x54, 0x4b, 0xf9
            ]
        );

        Ok(())
    }
}
