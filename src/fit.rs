use std::convert::Infallible;
use std::io::Write;
use std::ops::Add;
use std::sync::LazyLock;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use dimensioned::si::{M, Meter, MeterPerSecond, S, Second};
use num_traits::cast::NumCast;
use strum::EnumString;
use thiserror::Error;
use tracing::debug;
#[cfg(feature = "jsffi")]
use wasm_bindgen::prelude::*;

use crate::course::Course;
use crate::measure::{Centimeter, Millisecond, Nanosecond, SEMI, Semicircle};
use crate::types::{GeoPoint, TypeError};

/// The version of the Garmin SDK from which we obtain our profile information.
///
/// Represented in base 10 as two digits for the major version, followed by
/// three for the minor.
pub const PROFILE_VERSION: u16 = 21158;

/// An error when encoding to FIT
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum FitEncodeError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("Error encoding integer")]
    IntegerEncoding(#[from] std::num::TryFromIntError),
    #[error("Error in numeric cast")]
    NumCast,
    #[error("Error encoding string")]
    StringEncoding,
    #[error("Error encoding date_time")]
    DateTimeEncoding,
    #[error("Geographic computation error")]
    GeographicError(#[from] crate::geographic::GeographicError),
    #[error("Infallible")]
    Infallible(#[from] Infallible),
    #[error("Type error")]
    TypeError(#[from] TypeError),
}

type Result<T> = std::result::Result<T, FitEncodeError>;

fn write_string_field<W: Write>(s: &str, field_size: usize, w: &mut W) -> Result<()> {
    let st = truncate_to_char_boundary(s, field_size - 1);
    w.write_all(st.as_bytes())?;
    for _ in 0..(field_size - st.len()) {
        w.write_u8(0)?;
    }
    Ok(())
}

fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

static GARMIN_EPOCH: LazyLock<DateTime<Utc>> =
    LazyLock::new(|| "1989-12-31T00:00:00Z".parse::<DateTime<Utc>>().unwrap());

// The minimum value of a date_time as per the FIT global profile.  Values lower
// than this are to be interpreted as relative offsets rather than absolute
// times since the Garmin epoch.
const GARMIN_DATE_TIME_MIN: u32 = 0x10000000;

/// A date_time value as represented in a FIT file.
#[derive(Debug, Clone, Copy)]
struct FitDateTime {
    /// A timestamp as measured from the Garmin epoch of 1981-12-31T00:00:00Z,
    /// or a relative time in seconds if below 0x10000000.
    value_unsafe: u32,
}

impl TryFrom<DateTime<Utc>> for FitDateTime {
    type Error = FitEncodeError;

    fn try_from(value: DateTime<Utc>) -> std::result::Result<Self, Self::Error> {
        let ts = value.signed_duration_since(*GARMIN_EPOCH).num_seconds();
        if ts < (GARMIN_DATE_TIME_MIN as i64) {
            return Err(FitEncodeError::DateTimeEncoding);
        }
        Ok(Self {
            value_unsafe: u32::try_from(ts)?,
        })
    }
}

impl TryFrom<FitDateTime> for DateTime<Utc> {
    type Error = FitEncodeError;

    fn try_from(value: FitDateTime) -> std::result::Result<Self, Self::Error> {
        if value.value_unsafe < GARMIN_DATE_TIME_MIN {
            return Err(FitEncodeError::DateTimeEncoding);
        }
        Ok(GARMIN_EPOCH.add(TimeDelta::seconds(value.value_unsafe as i64)))
    }
}

impl TryFrom<TimeDelta> for Millisecond<u32> {
    type Error = FitEncodeError;

    fn try_from(value: TimeDelta) -> Result<Self> {
        let num_milliseconds =
            <u32 as NumCast>::from(value.num_milliseconds()).ok_or(FitEncodeError::NumCast)?;
        Ok(Self::new(num_milliseconds))
    }
}

/// A point on the surface of the ellipsoid, as represented in a FIT file.
#[derive(Debug, Clone, Copy)]
struct FitSurfacePoint {
    /// Latitude in semicircles
    lat: Semicircle<i32>,

    /// Longitude in semicircles
    lon: Semicircle<i32>,
}

impl TryFrom<GeoPoint> for FitSurfacePoint {
    type Error = FitEncodeError;

    fn try_from(value: GeoPoint) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            lat: value.lat().try_into()?,
            lon: value.lon().try_into()?,
        })
    }
}

impl TryFrom<FitSurfacePoint> for GeoPoint {
    type Error = FitEncodeError;

    fn try_from(value: FitSurfacePoint) -> std::result::Result<Self, Self::Error> {
        Ok(GeoPoint::new(value.lat.into(), value.lon.into(), None)?)
    }
}

/// Implements the Garmin FIT CRC algorithm.
///
/// A direct transcription of Garmin's reference implementation at
/// <https://developer.garmin.com/fit/protocol/>
struct Crc {
    sum: u16,
}

static CRC_TABLE: &[u16] = &[
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

/// A Write implementation that wraps another Write and computes a checksum over
/// data written.
struct CheckSummingWrite<'a, W: Write> {
    crc: Crc,
    base: &'a mut W,
    bytes_written: usize,
}

impl<'a, W: Write> CheckSummingWrite<'a, W> {
    fn new(base: &'a mut W) -> Self {
        Self {
            crc: Crc::new(),
            base,
            bytes_written: 0usize,
        }
    }

    /// Finish using the writer and write the CRC to the end of the stream.
    fn finish(self) -> Result<usize> {
        self.base.write_u16::<LittleEndian>(self.crc.sum)?;
        Ok(self.bytes_written)
    }
}

impl<W: Write> Write for CheckSummingWrite<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes_written += buf.len();
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
    CoursePoint = 32u16,
    FileCreator = 49u16,
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
        debug!(
            "Wrote definition frame for {:?} with local type {}",
            self.global_message, self.local_message_type
        );
        Ok(())
    }
}

/// Sport types
///
/// Names and numeric values manually copied from Profile.xlsx in FIT SDK
/// 21.171.00.
#[repr(u8)]
#[cfg_attr(feature = "cli", derive(strum::Display, clap::ValueEnum))]
#[derive(Clone, Copy, PartialEq, EnumString, Debug)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "cli", clap(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Sport {
    Generic = 0u8,
    Running = 1u8,
    Cycling = 2u8,
    Transition = 3u8, // Multisport transition
    FitnessEquipment = 4u8,
    Swimming = 5u8,
    Basketball = 6u8,
    Soccer = 7u8,
    Tennis = 8u8,
    AmericanFootball = 9u8,
    Training = 10u8,
    Walking = 11u8,
    CrossCountrySkiing = 12u8,
    AlpineSkiing = 13u8,
    Snowboarding = 14u8,
    Rowing = 15u8,
    Mountaineering = 16u8,
    Hiking = 17u8,
    Multisport = 18u8,
    Paddling = 19u8,
    Flying = 20u8,
    EBiking = 21u8,
    Motorcycling = 22u8,
    Boating = 23u8,
    Driving = 24u8,
    Golf = 25u8,
    HangGliding = 26u8,
    HorsebackRiding = 27u8,
    Hunting = 28u8,
    Fishing = 29u8,
    InlineSkating = 30u8,
    RockClimbing = 31u8,
    Sailing = 32u8,
    IceSkating = 33u8,
    SkyDiving = 34u8,
    Snowshoeing = 35u8,
    Snowmobiling = 36u8,
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

#[repr(u16)]
#[derive(Clone, Copy, Debug)]
enum FileManufacturer {
    Development = 255,
}

struct FileIdMessage<'a> {
    file_type: FileType,
    manufacturer: FileManufacturer,
    time_created: FitDateTime,
    product_name: &'a str,
}

impl<'a> FileIdMessage<'a> {
    fn new(
        file_type: FileType,
        manufacturer: FileManufacturer,
        time_created: FitDateTime,
        product_name: &'a str,
    ) -> Self {
        Self {
            file_type,
            manufacturer,
            time_created,
            product_name,
        }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(0, 1, 0),   // type
            FieldDefinition::new(1, 2, 132), // manufacturer
            FieldDefinition::new(4, 4, 134), // time_created
            FieldDefinition::new(8, 14, 7),  // product_name
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u8(self.file_type as u8)?;
        w.write_u16::<BigEndian>(self.manufacturer as u16)?;
        w.write_u32::<BigEndian>(self.time_created.value_unsafe)?;
        write_string_field(self.product_name, 14, w)?;
        Ok(())
    }
}

struct LapMessage {
    start_time: FitDateTime,
    duration: Millisecond<u32>,
    distance: Centimeter<u32>,
    start_pos: Option<FitSurfacePoint>,
    end_pos: Option<FitSurfacePoint>,
}

impl LapMessage {
    fn new(
        start_time: FitDateTime,
        duration: Millisecond<u32>,
        distance: Centimeter<u32>,
        start_pos: Option<FitSurfacePoint>,
        end_pos: Option<FitSurfacePoint>,
    ) -> Self {
        Self {
            start_time,
            duration,
            distance,
            start_pos,
            end_pos,
        }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(2, 4, 134),   // start_time
            FieldDefinition::new(253, 4, 134), // timestamp
            FieldDefinition::new(7, 4, 134),   // total_elapsed_time
            FieldDefinition::new(8, 4, 134),   // total_timer_time
            FieldDefinition::new(9, 4, 134),   // total_distance
            FieldDefinition::new(3, 4, 133),   // start_position_lat
            FieldDefinition::new(4, 4, 133),   // start_position_long
            FieldDefinition::new(5, 4, 133),   // end_position_lat
            FieldDefinition::new(6, 4, 133),   // end_position_long
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u32::<BigEndian>(self.start_time.value_unsafe)?;
        w.write_u32::<BigEndian>(self.start_time.value_unsafe)?;
        w.write_u32::<BigEndian>(self.duration.value_unsafe)?;
        w.write_u32::<BigEndian>(self.duration.value_unsafe)?;
        w.write_u32::<BigEndian>(self.distance.value_unsafe)?;
        let null_pos = FitSurfacePoint {
            lat: 0 * SEMI,
            lon: 0 * SEMI,
        };
        let start_pos = self.start_pos.unwrap_or(null_pos);
        let end_pos = self.end_pos.unwrap_or(null_pos);
        w.write_i32::<BigEndian>(start_pos.lat.value_unsafe)?;
        w.write_i32::<BigEndian>(start_pos.lon.value_unsafe)?;
        w.write_i32::<BigEndian>(end_pos.lat.value_unsafe)?;
        w.write_i32::<BigEndian>(end_pos.lon.value_unsafe)?;
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
    event_group: u8,
}

impl EventMessage {
    fn new(event: Event, event_type: EventType, timestamp: FitDateTime, event_group: u8) -> Self {
        Self {
            event,
            event_type,
            timestamp,
            event_group,
        }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(253, 4, 134), // timestamp
            FieldDefinition::new(0, 1, 0),     // event
            FieldDefinition::new(4, 1, 2),     // event_group
            FieldDefinition::new(1, 1, 0),     // event_type
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u32::<BigEndian>(self.timestamp.value_unsafe)?;
        w.write_u8(self.event as u8)?;
        w.write_u8(self.event_group)?;
        w.write_u8(self.event_type as u8)?;
        Ok(())
    }
}

struct RecordMessage {
    /// The record's position on the surface of the ellipsoid.
    position: FitSurfacePoint,

    /// The record's cumulative distance along the entire course.
    cumulative_distance: Centimeter<u32>,

    /// The absolute time of the record.
    timestamp: FitDateTime,
}

impl RecordMessage {
    fn new(
        position: FitSurfacePoint,
        cumulative_distance: Centimeter<u32>,
        timestamp: FitDateTime,
    ) -> Self {
        Self {
            position,
            cumulative_distance,
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
        w.write_i32::<BigEndian>(self.position.lat.value_unsafe)?;
        w.write_i32::<BigEndian>(self.position.lon.value_unsafe)?;
        w.write_u32::<BigEndian>(self.cumulative_distance.value_unsafe)?;
        w.write_u32::<BigEndian>(self.timestamp.value_unsafe)?;
        Ok(())
    }
}

/// Course point types
///
/// Names and numeric values manually copied from Profile.xlsx in FIT SDK
/// 21.158.00.  See
/// [docs/point_types.md](https://github.com/mshroyer/coursepointer/blob/main/docs/point_types.md)
/// for how these may appear on devices in practice.  The `Generic` variant
/// typically renders as a pin or a flag icon.
#[repr(u8)]
#[cfg_attr(feature = "cli", derive(strum::Display, clap::ValueEnum))]
#[derive(Clone, Copy, PartialEq, EnumString, Debug)]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "cli", clap(rename_all = "snake_case"))]
#[non_exhaustive]
#[cfg_attr(feature = "jsffi", wasm_bindgen)]
pub enum CoursePointType {
    Generic = 0u8,
    Summit = 1u8,
    Valley = 2u8,
    Water = 3u8,
    Food = 4u8,
    Danger = 5u8,
    Left = 6u8,
    Right = 7u8,
    Straight = 8u8,
    FirstAid = 9u8,
    FourthCategory = 10u8,
    ThirdCategory = 11u8,
    SecondCategory = 12u8,
    FirstCategory = 13u8,
    HorsCategory = 14u8,
    Sprint = 15u8,
    LeftFork = 16u8,
    RightFork = 17u8,
    MiddleFork = 18u8,
    SlightLeft = 19u8,
    SharpLeft = 20u8,
    SlightRight = 21u8,
    SharpRight = 22u8,
    UTurn = 23u8,
    SegmentStart = 24u8,
    SegmentEnd = 25u8,
    Campsite = 27u8,
    AidStation = 28u8,
    RestArea = 29u8,
    GeneralDistance = 30u8, // Used with UpAhead
    Service = 31u8,
    EnergyGel = 32u8,
    SportsDrink = 33u8,
    MileMarker = 34u8,
    Checkpoint = 35u8,
    Shelter = 36u8,
    MeetingSpot = 37u8,
    Overlook = 38u8,
    Toilet = 39u8,
    Shower = 40u8,
    Gear = 41u8,
    SharpCurve = 42u8,
    SteepIncline = 43u8,
    Tunnel = 44u8,
    Bridge = 45u8,
    Obstacle = 46u8,
    Crossing = 47u8,
    Store = 48u8,
    Transition = 49u8,
    Navaid = 50u8,
    Transport = 51u8,
    Alert = 52u8,
    Info = 53u8,
}

struct CoursePointMessage {
    timestamp: FitDateTime,
    type_: CoursePointType,
    position: FitSurfacePoint,
    distance: Centimeter<u32>,
    name: String,
}

impl CoursePointMessage {
    fn new(
        timestamp: FitDateTime,
        type_: CoursePointType,
        position: FitSurfacePoint,
        distance: Centimeter<u32>,
        name: String,
    ) -> Self {
        Self {
            timestamp,
            type_,
            position,
            distance,
            name,
        }
    }

    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(1, 4, 134), // timestamp
            FieldDefinition::new(2, 4, 133), // lat
            FieldDefinition::new(3, 4, 133), // lon
            FieldDefinition::new(4, 4, 134), // distance
            FieldDefinition::new(5, 1, 0),   // type
            FieldDefinition::new(6, 16, 7),  // name
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u32::<BigEndian>(self.timestamp.value_unsafe)?;
        w.write_i32::<BigEndian>(self.position.lat.value_unsafe)?;
        w.write_i32::<BigEndian>(self.position.lon.value_unsafe)?;
        w.write_u32::<BigEndian>(self.distance.value_unsafe)?;
        w.write_u8(self.type_ as u8)?;
        write_string_field(self.name.as_str(), 16, w)?;
        Ok(())
    }
}

struct FileCreatorMessage {
    software_version: u16,
    hardware_version: u8,
}

impl FileCreatorMessage {
    fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition::new(0, 2, 132), // software_version
            FieldDefinition::new(1, 1, 2),   // hardware_version
        ]
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_u16::<BigEndian>(self.software_version)?;
        w.write_u8(self.hardware_version)?;
        Ok(())
    }
}

fn timedelta_from_seconds(s: Second<f64>) -> Result<TimeDelta> {
    Ok(TimeDelta::nanoseconds(
        Nanosecond::<i64>::num_cast_from(s.into())
            .ok_or(FitEncodeError::NumCast)?
            .value_unsafe,
    ))
}

/// Options for writing a FIT course
#[derive(Clone, Debug)]
pub struct FitCourseOptions {
    speed: MeterPerSecond<f64>,
    start_time: DateTime<Utc>,
    sport: Sport,
    product_name: String,
    software_version: u16,
    hardware_version: u8,
}

impl FitCourseOptions {
    /// Write the FIT file using the given speed for record timestamps
    ///
    /// This has the effect of setting the speed of the Virtual Partner on
    /// compatible Garmin devices.
    pub fn with_speed(mut self, speed: MeterPerSecond<f64>) -> Self {
        self.speed = speed;
        self
    }

    /// Set the timestamp at which the course starts
    ///
    /// Controls the timestamps on lap and record messages.  An arbitrary, but
    /// consistent and reproducible, time will be used if left unset.
    pub fn with_start_time(mut self, start_time: DateTime<Utc>) -> Self {
        self.start_time = start_time;
        self
    }

    /// Set the course's sport
    ///
    /// Defaults to `cycling` if unset.
    pub fn with_sport(mut self, sport: Sport) -> Self {
        self.sport = sport;
        self
    }

    /// Set the product name to encode
    ///
    /// The first 13 bytes of this string will go in the `file_id` message's
    /// `product_name` field.  Defaults to the empty string if unset.
    pub fn with_product_name(mut self, product_name: String) -> Self {
        self.product_name = product_name;
        self
    }

    /// Set the software version to encode
    ///
    /// This goes in the `file_creator` message's `software_version` field. Zero
    /// by default.
    pub fn with_software_version(mut self, software_version: u16) -> Self {
        self.software_version = software_version;
        self
    }

    /// Set the hardware version to encode
    ///
    /// This goes in the `file_creator` message's `hardware_version` field. Zero
    /// by default.
    pub fn with_hardware_version(mut self, hardware_version: u8) -> Self {
        self.hardware_version = hardware_version;
        self
    }
}

impl Default for FitCourseOptions {
    fn default() -> Self {
        Self {
            speed: 8.0 * M / S,
            // Defaulting to Utc::now() would mean FIT writes aren't
            // reproducible, so let's arbitrarily go with my niece's birthday as
            // a consistent default.
            start_time: Utc.with_ymd_and_hms(2019, 11, 23, 00, 00, 00).unwrap(),
            sport: Sport::Cycling,
            product_name: "".to_owned(),
            software_version: 0u16,
            hardware_version: 0u8,
        }
    }
}

/// A write-only Garmin FIT course file
pub struct CourseFile<'a> {
    course: &'a Course,
    options: FitCourseOptions,
}

impl<'a> CourseFile<'a> {
    /// Creates a new course file
    ///
    /// `start_time` and `speed` together determine the timestamps of the
    /// records that will be written to the course file.
    pub fn new(course: &'a Course, options: FitCourseOptions) -> Self {
        Self { course, options }
    }

    /// Encode and write the course file
    #[tracing::instrument(name = "encode_fit", level = "debug", skip_all)]
    pub fn encode<W: Write>(&self, mut w: W) -> Result<()> {
        // File header
        let mut hw = CheckSummingWrite::new(&mut w);
        let h = FileHeader::new(self.get_data_size())?;
        h.encode(&mut hw)?;
        let bytes_written = hw.finish()?;
        debug!("Wrote {} file header bytes + 2 byte CRC", bytes_written);

        // File data
        let mut dw = CheckSummingWrite::new(&mut w);

        // TODO: Add software info to file_id, maybe file_creator messages
        DefinitionFrame::new(
            GlobalMessage::FileId,
            0u8,
            FileIdMessage::field_definitions(),
        )
        .encode(&mut dw)?;
        FileIdMessage::new(
            FileType::Course,
            FileManufacturer::Development,
            FitDateTime::try_from(self.options.start_time)?,
            self.options.product_name.as_str(),
        )
        .encode(0u8, &mut dw)?;

        DefinitionFrame::new(
            GlobalMessage::Course,
            1u8,
            CourseMessage::field_definitions(),
        )
        .encode(&mut dw)?;
        CourseMessage::new(
            self.course
                .name
                .clone()
                .unwrap_or("Untitled course".to_owned()),
            self.options.sport,
        )
        .encode(1u8, &mut dw)?;

        let start_pos = self
            .course
            .records
            .first()
            .map(|r| r.point.try_into())
            .transpose()?;
        let end_pos = self
            .course
            .records
            .last()
            .map(|r| r.point.try_into())
            .transpose()?;
        DefinitionFrame::new(GlobalMessage::Lap, 2u8, LapMessage::field_definitions())
            .encode(&mut dw)?;
        LapMessage::new(
            FitDateTime::try_from(self.options.start_time)?,
            self.total_duration()?.try_into()?,
            Centimeter::<u32>::num_cast_from(self.course.total_distance().into())
                .ok_or(FitEncodeError::NumCast)?,
            start_pos,
            end_pos,
        )
        .encode(2u8, &mut dw)?;

        DefinitionFrame::new(GlobalMessage::Event, 3u8, EventMessage::field_definitions())
            .encode(&mut dw)?;
        EventMessage::new(
            Event::Timer,
            EventType::Start,
            FitDateTime::try_from(self.options.start_time)?,
            0,
        )
        .encode(3u8, &mut dw)?;

        DefinitionFrame::new(
            GlobalMessage::Record,
            4u8,
            RecordMessage::field_definitions(),
        )
        .encode(&mut dw)?;
        for record in &self.course.records {
            let distance: Centimeter<f64> = record.cumulative_distance.into();
            let timedelta: Second<f64> = record.cumulative_distance / self.options.speed;
            let timestamp = self
                .options
                .start_time
                .add(timedelta_from_seconds(timedelta)?);
            let record_message = RecordMessage::new(
                record.point.try_into()?,
                Centimeter::<u32>::num_cast_from(distance).ok_or(FitEncodeError::NumCast)?,
                timestamp.try_into()?,
            );
            record_message.encode(4u8, &mut dw)?;
        }
        debug!(
            "Encoded {} course record messages",
            self.course.records.len()
        );

        DefinitionFrame::new(
            GlobalMessage::CoursePoint,
            5u8,
            CoursePointMessage::field_definitions(),
        )
        .encode(&mut dw)?;
        for course_point in &self.course.course_points {
            let distance: Centimeter<f64> = course_point.distance.into();
            let timedelta: Second<f64> = course_point.distance / self.options.speed;
            let timestamp = self
                .options
                .start_time
                .add(timedelta_from_seconds(timedelta)?);
            let course_point_message = CoursePointMessage::new(
                timestamp.try_into()?,
                course_point.point_type,
                course_point.point.try_into()?,
                Centimeter::<u32>::num_cast_from(distance).ok_or(FitEncodeError::NumCast)?,
                course_point.name.to_string(),
            );
            course_point_message.encode(5u8, &mut dw)?;
        }
        debug!(
            "Encoded {} course point messages",
            self.course.course_points.len()
        );

        EventMessage::new(
            Event::Timer,
            EventType::Stop,
            FitDateTime::try_from(self.options.start_time.add(self.total_duration()?))?,
            0,
        )
        .encode(3u8, &mut dw)?;

        DefinitionFrame::new(
            GlobalMessage::FileCreator,
            6u8,
            FileCreatorMessage::field_definitions(),
        )
        .encode(&mut dw)?;
        FileCreatorMessage {
            software_version: self.options.software_version,
            hardware_version: self.options.hardware_version,
        }
        .encode(6u8, &mut dw)?;

        let bytes_written = dw.finish()?;
        debug!("Wrote {bytes_written} data bytes + 2 byte CRC");
        w.flush()?;
        debug!("Flushed base writer");
        Ok(())
    }

    fn total_distance(&self) -> Meter<f64> {
        self.course
            .records
            .iter()
            .last()
            .map(|r| r.cumulative_distance)
            .unwrap_or(0.0 * M)
    }

    /// Returns the timestamp corresponding to the course's speed and total
    /// distance.
    fn total_duration(&self) -> Result<TimeDelta> {
        self.total_duration_at_distance(self.total_distance())
    }

    fn total_duration_at_distance(&self, distance: Meter<f64>) -> Result<TimeDelta> {
        timedelta_from_seconds(distance / self.options.speed)
    }

    /// Computes the total size of the data segment of this file, including
    /// definition messages and data messages.
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
        sz += self.course.records.len()
            * CourseFile::get_data_message_size(RecordMessage::field_definitions());

        sz +=
            CourseFile::get_definition_message_size(CoursePointMessage::field_definitions().len());
        sz += self.course.course_points.len()
            * CourseFile::get_data_message_size(CoursePointMessage::field_definitions());

        sz +=
            CourseFile::get_definition_message_size(FileCreatorMessage::field_definitions().len());
        sz += CourseFile::get_data_message_size(FileCreatorMessage::field_definitions());

        debug!("Computed FIT data (definition + messages) size: {}", sz);

        sz
    }

    /// Computes the size of a definition message based on the number of field
    /// definitions, assuming no developer data fields.
    fn get_definition_message_size(num_defs: usize) -> usize {
        6usize + 3 * num_defs
    }

    /// Computes the size of a single instance of a data message, given its
    /// field definitions.
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
        // The CRC value from the last two bytes of the header, interpreted as little
        // endian.
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
