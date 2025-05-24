use std::convert::Infallible;
use std::io::Write;
use std::ops::Add;
use std::sync::LazyLock;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use chrono::{DateTime, TimeDelta, Utc};
use num_traits::Pow;
use num_traits::bounds::Bounded;
use num_traits::cast::NumCast;
use num_traits::float::Float;
use num_traits::int::PrimInt;
use thiserror::Error;

use geographic::SurfacePoint;

use crate::measure::{Centimeters, Degrees, Meters, MetersPerSecond};

/// The version of the Garmin SDK from which we obtain our profile information.
///
/// Represented in base 10 as two digits for the major version, followed by three for the minor.
pub const PROFILE_VERSION: u16 = 21158;

#[derive(Error, Debug)]
pub enum FitEncodeError {
    #[error("writing to output")]
    Write(#[from] std::io::Error),
    #[error("encoding integer")]
    IntegerEncoding(#[from] std::num::TryFromIntError),
    #[error("float conversion")]
    FloatConversion,
    #[error("encoding string")]
    StringEncoding,
    #[error("encoding date_time")]
    DateTimeEncoding,
    #[error("geographiclib error: {0}")]
    GeographicLibError(String),
    #[error("infallible")]
    Infallible(#[from] Infallible),
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

static GARMIN_EPOCH: LazyLock<DateTime<Utc>> =
    LazyLock::new(|| "1989-12-31T00:00:00Z".parse::<DateTime<Utc>>().unwrap());

// The minimum value of a date_time as per the FIT global profile.  Values lower than this are to
// be interpreted as relative offsets rather than absolute times since the Garmin epoch.
const GARMIN_DATE_TIME_MIN: u32 = 0x10000000;

/// A date_time value as represented in a FIT file.
#[derive(Debug, Clone, Copy)]
struct FitDateTime {
    /// A timestamp as measured from the Garmin epoch of 1981-12-31T00:00:00Z, or a relative time
    /// in seconds if below 0x10000000.
    value: u32,
}

impl TryFrom<DateTime<Utc>> for FitDateTime {
    type Error = FitEncodeError;

    fn try_from(value: DateTime<Utc>) -> std::result::Result<Self, Self::Error> {
        let ts = value.signed_duration_since(*GARMIN_EPOCH).num_seconds();
        if ts < (GARMIN_DATE_TIME_MIN as i64) {
            return Err(FitEncodeError::DateTimeEncoding);
        }
        Ok(Self {
            value: u32::try_from(ts)?,
        })
    }
}

impl TryFrom<FitDateTime> for DateTime<Utc> {
    type Error = FitEncodeError;

    fn try_from(value: FitDateTime) -> std::result::Result<Self, Self::Error> {
        if value.value < GARMIN_DATE_TIME_MIN {
            return Err(FitEncodeError::DateTimeEncoding);
        }
        Ok(GARMIN_EPOCH.add(TimeDelta::seconds(value.value as i64)))
    }
}

/// A point on the surface of the ellipsoid, as represented in a FIT file.
#[derive(Debug, Clone, Copy)]
struct FitSurfacePoint {
    /// Latitutde in semicircles
    lat_semis: i32,

    /// Longitude in semicircles
    lon_semis: i32,
}

/// Lossily converts a float to an integer type
///
/// Accepts lossy conversion, but still returns an error if the value to be converted lies outside
/// the expressible range of the target integer type.
fn truncate_float<F, I>(f: F) -> Result<I>
where
    F: Float + NumCast,
    I: PrimInt + NumCast + Bounded,
{
    let min = NumCast::from(I::min_value()).unwrap();
    let max = NumCast::from(I::max_value()).unwrap();
    if f >= min && f <= max {
        Ok(NumCast::from(f.round()).unwrap())
    } else {
        Err(FitEncodeError::FloatConversion)
    }
}

impl TryFrom<SurfacePoint> for FitSurfacePoint {
    type Error = FitEncodeError;

    fn try_from(value: SurfacePoint) -> std::result::Result<Self, Self::Error> {
        let lat_semis = truncate_float((2f64.pow(31) / 180.0) * value.lat)?;
        let lon_semis = truncate_float((2f64.pow(31) / 180.0) * value.lon)?;
        Ok(Self {
            lat_semis,
            lon_semis,
        })
    }
}

impl TryFrom<FitSurfacePoint> for SurfacePoint {
    type Error = FitEncodeError;

    fn try_from(value: FitSurfacePoint) -> std::result::Result<Self, Self::Error> {
        let lat = <f64 as From<i32>>::from(value.lat_semis) * 180.0 / 2f64.pow(31);
        let lon = <f64 as From<i32>>::from(value.lon_semis) * 180.0 / 2f64.pow(31);
        Ok(Self { lat, lon })
    }
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
    data_size: u32,
}

impl FileHeader {
    pub fn new(data_size: usize) -> Result<Self> {
        let data_size_u32 = u32::try_from(data_size)?;
        Ok(Self {
            protocol_version: ProtocolVersion::V10,
            data_size: data_size_u32,
        })
    }

    pub fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u8(14)?;
        w.write_u8(self.protocol_version as u8)?;
        w.write_u16::<LittleEndian>(PROFILE_VERSION)?;
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
    Lap = 19u16,
    Record = 20u16,
    Event = 21u16,
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

struct LapMessage {
    start_time: FitDateTime,
    duration_ms: u32,
    dist_cm: u32,
    start_pos: Option<FitSurfacePoint>,
    end_pos: Option<FitSurfacePoint>,
}

impl LapMessage {
    fn new(
        start_time: FitDateTime,
        duration_ms: u32,
        dist_cm: u32,
        start_pos: Option<FitSurfacePoint>,
        end_pos: Option<FitSurfacePoint>,
    ) -> Self {
        Self {
            start_time,
            duration_ms,
            dist_cm,
            start_pos,
            end_pos,
        }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(2, 4, 134), // start_time
            FieldDefinition::new(7, 4, 134), // total_elapsed_time
            FieldDefinition::new(8, 4, 134), // total_timer_time
            FieldDefinition::new(9, 4, 134), // total_distance
            FieldDefinition::new(3, 4, 133), // start_position_lat
            FieldDefinition::new(4, 4, 133), // start_position_long
            FieldDefinition::new(5, 4, 133), // end_position_lat
            FieldDefinition::new(6, 4, 133), // end_position_long
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u32::<BigEndian>(self.start_time.value)?;
        w.write_u32::<BigEndian>(self.duration_ms)?;
        w.write_u32::<BigEndian>(self.duration_ms)?;
        w.write_u32::<BigEndian>(self.dist_cm)?;
        let null_pos = FitSurfacePoint {
            lat_semis: 0i32,
            lon_semis: 0i32,
        };
        let start_pos = self.start_pos.unwrap_or(null_pos);
        let end_pos = self.end_pos.unwrap_or(null_pos);
        w.write_i32::<BigEndian>(start_pos.lat_semis)?;
        w.write_i32::<BigEndian>(start_pos.lon_semis)?;
        w.write_i32::<BigEndian>(end_pos.lat_semis)?;
        w.write_i32::<BigEndian>(end_pos.lon_semis)?;
        Ok(())
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum Event {
    Timer = 0u8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
enum EventType {
    Start = 0u8,
    Stop = 1u8,
}

struct EventMessage {
    event: Event,
    event_type: EventType,
    timestamp: FitDateTime,
}

impl EventMessage {
    fn new(event: Event, event_type: EventType, timestamp: FitDateTime) -> Self {
        Self {
            event,
            event_type,
            timestamp,
        }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(0, 1, 0),     // event
            FieldDefinition::new(1, 1, 0),     // event_type
            FieldDefinition::new(253, 4, 134), // timestamp
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u8(self.event as u8)?;
        w.write_u8(self.event_type as u8)?;
        w.write_u32::<BigEndian>(self.timestamp.value)?;
        Ok(())
    }
}

struct RecordMessage {
    /// The record's position on the surface of the ellipsoid.
    position: FitSurfacePoint,

    /// The distance in cm from the previous record.
    dist_cm: u32,

    /// The absolute time of the record.
    timestamp: FitDateTime,
}

impl RecordMessage {
    fn new(position: FitSurfacePoint, dist_cm: u32, timestamp: FitDateTime) -> Self {
        Self {
            position,
            dist_cm,
            timestamp,
        }
    }

    // TODO: Proc macro for deriving field definitions + maybe encoding too?
    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(0, 4, 133),   // lat
            FieldDefinition::new(1, 4, 133),   // lon
            FieldDefinition::new(5, 4, 134),   // distance
            FieldDefinition::new(253, 4, 134), // timestamp
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_i32::<BigEndian>(self.position.lat_semis)?;
        w.write_i32::<BigEndian>(self.position.lon_semis)?;
        w.write_u32::<BigEndian>(self.dist_cm)?;
        w.write_u32::<BigEndian>(self.timestamp.value)?;
        Ok(())
    }
}

pub struct CourseFile {
    name: String,
    start_time: DateTime<Utc>,
    speed: MetersPerSecond<f64>,
    records: Vec<RecordMessage>,
    total_distance: Meters<f64>,
    last_record_added: Option<SurfacePoint>,
}

impl CourseFile {
    pub fn new(
        name: String,
        start_time: DateTime<Utc>,
        speed: MetersPerSecond<f64>,
    ) -> Self {
        Self {
            name,
            start_time,
            speed,
            records: vec![],
            total_distance: Meters(0.0),
            last_record_added: None,
        }
    }

    pub fn add_record(&mut self, point: SurfacePoint) -> Result<()> {
        let incremental_distance = match self.last_record_added {
            None => Meters(0.0),
            Some(prev_point) => {
                let sln = geographic::inverse(&prev_point, &point)
                    .or_else(|s| Err(FitEncodeError::GeographicLibError(s)))?;
                Meters(sln.meters)
            }
        };
        self.total_distance += incremental_distance;
        self.records.push(RecordMessage::new(
            FitSurfacePoint::try_from(point)?,
            truncate_float(Centimeters::from(self.total_distance).0)?,
            FitDateTime::try_from(self.start_time.add(self.total_duration()?))?,
        ));
        Ok(())
    }

    pub fn encode<W: Write>(&self, w: &mut W) -> Result<()> {
        // File header
        let mut hw = CheckSummingWrite::new(w);
        let h = FileHeader::new(self.get_data_size())?;
        h.encode(&mut hw)?;
        hw.finish()?;

        // File data
        let mut dw = CheckSummingWrite::new(w);

        // TODO: Add software info to file_id, maybe file_creator messages
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

        let start_pos = self.records.iter().map(|r| r.position).next();
        let end_pos = self.records.iter().map(|r| r.position).last();
        DefinitionFrame::new(GlobalMessage::Lap, 2u8, LapMessage::field_definitions())
            .encode(&mut dw)?;
        LapMessage::new(
            FitDateTime::try_from(self.start_time)?,
            u32::try_from(self.total_duration()?.num_milliseconds())?,
            truncate_float(Centimeters::from(self.total_distance).0)?,
            start_pos,
            end_pos,
        )
        .encode(2u8, &mut dw)?;

        DefinitionFrame::new(GlobalMessage::Event, 3u8, EventMessage::field_definitions())
            .encode(&mut dw)?;
        EventMessage::new(
            Event::Timer,
            EventType::Start,
            FitDateTime::try_from(self.start_time)?,
        )
        .encode(3u8, &mut dw)?;

        DefinitionFrame::new(GlobalMessage::Record, 4u8, RecordMessage::field_definitions())
            .encode(&mut dw)?;
        for record in &self.records {
            record.encode(4u8, &mut dw)?;
        }

        EventMessage::new(
            Event::Timer,
            EventType::Stop,
            FitDateTime::try_from(self.start_time.add(self.total_duration()?))?,
        )
        .encode(3u8, &mut dw)?;
        dw.finish()?;

        Ok(())
    }

    /// Returns the timestamp corresponding to the course's speed and total distance.
    fn total_duration(&self) -> Result<TimeDelta> {
        let total_duration_seconds: i64 = truncate_float((self.total_distance / self.speed).0)?;
        Ok(TimeDelta::seconds(total_duration_seconds))
    }

    /// Computes the total size of the data segment of this file, including definition messages
    /// and data messages.
    fn get_data_size(&self) -> usize {
        let mut sz = 0usize;

        // TODO: Abstract out message definition encoding
        sz += CourseFile::get_definition_message_size(FileIdMessage::field_definitions().len());
        sz += CourseFile::get_data_message_size(FileIdMessage::field_definitions());

        sz += CourseFile::get_definition_message_size(CourseMessage::field_definitions().len());
        sz += CourseFile::get_data_message_size(CourseMessage::field_definitions());

        sz += CourseFile::get_definition_message_size(LapMessage::field_definitions().len());
        sz += CourseFile::get_data_message_size(LapMessage::field_definitions());

        sz += CourseFile::get_definition_message_size(EventMessage::field_definitions().len());
        sz += 2 * CourseFile::get_data_message_size(EventMessage::field_definitions());

        sz += CourseFile::get_definition_message_size(RecordMessage::field_definitions().len());
        sz += self.records.len()
            * CourseFile::get_data_message_size(RecordMessage::field_definitions());

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
    use super::{CheckSummingWrite, Crc, FileHeader, Result};

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
        let mut cw = CheckSummingWrite::new(&mut buf);
        let header = FileHeader::new(17032usize)?;
        header.encode(&mut cw)?;
        cw.finish()?;

        assert_eq!(
            buf,
            &[
                0x0e, 0x10, 0xa6, 0x52, 0x88, 0x42, 0x00, 0x00, 0x2e, 0x46, 0x49, 0x54, 0x0b, 0xb9,
            ]
        );

        Ok(())
    }
}
