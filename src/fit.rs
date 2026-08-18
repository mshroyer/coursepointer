use std::convert::Infallible;
use std::io::Write;
use std::ops::Add;
use std::sync::LazyLock;

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use chrono::{DateTime, TimeDelta, Utc};
use dimensioned::si::{M, Meter, MeterPerSecond, S, Second};
use num_traits::cast::NumCast;
#[cfg(feature = "jsffi")]
use serde::Serialize;
use strum::EnumString;
use thiserror::Error;
use tracing::{debug, info, warn};
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

/// The `uint16` value the FIT profile reserves to mean "unknown".
const UINT16_INVALID: u16 = 0xFFFF;

/// The FIT profile's scale for the record message's `altitude` field.
const ALTITUDE_SCALE: f64 = 5.0;

/// The FIT profile's offset, in meters, for the record message's `altitude`
/// field.
const ALTITUDE_OFFSET: f64 = 500.0;

/// An altitude as represented in a FIT file.
///
/// The profile stores the record message's `altitude` as a `uint16` with a
/// scale of 5 and an offset of 500 meters, so it covers altitudes from -500 m
/// up to about 12,606 m at a resolution of 20 cm.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FitAltitude {
    /// The scaled and offset altitude.
    value_unsafe: u16,
}

impl FitAltitude {
    /// Converts an elevation into its FIT representation.
    ///
    /// Returns `None` if the elevation is outside the range the profile can
    /// represent.  We never encode [`UINT16_INVALID`] to mean "unknown" here:
    /// while the FIT SDK reads that back as missing data, Garmin Edge and
    /// fenix devices display it as a literal altitude of about 12,607 m.
    fn new(elevation: Meter<f64>) -> Option<Self> {
        let scaled = (elevation.value_unsafe + ALTITUDE_OFFSET) * ALTITUDE_SCALE;
        match <u16 as NumCast>::from(scaled.round()) {
            Some(v) if v != UINT16_INVALID => Some(Self { value_unsafe: v }),
            _ => {
                debug!("Elevation {elevation} is out of the FIT altitude field's range");
                None
            }
        }
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
#[cfg_attr(
    feature = "jsffi",
    derive(strum::EnumIter, strum::Display, num_enum::TryFromPrimitive)
)]
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

impl Default for Sport {
    fn default() -> Self {
        Sport::Generic
    }
}

struct CourseMessage {
    name: String,
    sport: Sport,
}

impl CourseMessage {
    fn new(name: String, sport: Sport) -> Self {
        info!("sport: {sport:?}");
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

    /// The record's altitude.
    ///
    /// `None` if the course being encoded isn't carrying elevation data, in
    /// which case the field is left out of the message definition.  Either all
    /// of a course's records have an altitude or none of them do.
    altitude: Option<FitAltitude>,
}

impl RecordMessage {
    fn new(
        position: FitSurfacePoint,
        cumulative_distance: Centimeter<u32>,
        timestamp: FitDateTime,
        altitude: Option<FitAltitude>,
    ) -> Self {
        Self {
            position,
            cumulative_distance,
            timestamp,
            altitude,
        }
    }

    // TODO: Proc macro for deriving field definitions + maybe encoding too?
    fn field_definitions(with_altitude: bool) -> Vec<FieldDefinition> {
        let mut defs = vec![
            FieldDefinition::new(0, 4, 133),   // lat
            FieldDefinition::new(1, 4, 133),   // lon
            FieldDefinition::new(5, 4, 134),   // distance
            FieldDefinition::new(253, 4, 134), // timestamp
        ];
        if with_altitude {
            defs.push(FieldDefinition::new(2, 2, 132)); // altitude
        }
        defs
    }

    fn encode<W: Write>(&self, local_message_id: u8, w: &mut W) -> Result<()> {
        w.write_u8(local_message_id & 0x0F)?;
        w.write_i32::<BigEndian>(self.position.lat.value_unsafe)?;
        w.write_i32::<BigEndian>(self.position.lon.value_unsafe)?;
        w.write_u32::<BigEndian>(self.cumulative_distance.value_unsafe)?;
        w.write_u32::<BigEndian>(self.timestamp.value_unsafe)?;
        if let Some(altitude) = self.altitude {
            w.write_u16::<BigEndian>(altitude.value_unsafe)?;
        }
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
#[cfg_attr(feature = "jsffi", derive(Serialize))]
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
    elevation: bool,
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
    /// Defaults to `generic` if unset.
    pub fn with_sport(mut self, sport: Sport) -> Self {
        self.sport = sport;
        self
    }

    /// Set whether to write the course's elevation data
    ///
    /// When enabled, which is the default, records carry the elevations from
    /// the input as their `altitude` field.  This only happens if every record
    /// has an elevation the field can represent; otherwise the course is
    /// written without altitudes either way.
    pub fn with_elevation(mut self, elevation: bool) -> Self {
        self.elevation = elevation;
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
            // Some devices may use the file_id message, including its
            // time_created field, as a key to uniquely identify courses, so it
            // isn't safe to default to a fixed timestamp.  Using the current
            // time mimics the behavior of Garmin Connect.
            //
            // See ANT+'s "FIT File Types Description" document, revision 2.2,
            // which states: "If the combination of type, manufacturer, product
            // and serial_number is insufficient, [...] the time_created or
            // number fields must be populated to differentiate the files."
            start_time: Utc::now(),
            sport: Sport::Cycling,
            product_name: "".to_owned(),
            software_version: 0u16,
            hardware_version: 0u8,
            elevation: true,
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
        let altitudes = self.record_altitudes();

        // File header
        let mut hw = CheckSummingWrite::new(&mut w);
        let h = FileHeader::new(self.get_data_size(altitudes.is_some()))?;
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
            RecordMessage::field_definitions(altitudes.is_some()),
        )
        .encode(&mut dw)?;
        for (i, record) in self.course.records.iter().enumerate() {
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
                altitudes.as_ref().map(|a| a[i]),
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

    /// The altitudes to write to the course's record messages
    ///
    /// Returns `None`, meaning the altitude field is left out of the record
    /// definition entirely, unless elevation is enabled in the options and
    /// every record has an elevation the FIT profile's altitude field can
    /// represent.  It's all or nothing because devices don't reliably read an
    /// individual altitude set to the profile's invalid value as unknown.
    fn record_altitudes(&self) -> Option<Vec<FitAltitude>> {
        if !self.options.elevation {
            return None;
        }

        let altitudes = self
            .course
            .records
            .iter()
            .map(|r| r.point.ele().and_then(FitAltitude::new))
            .collect::<Option<Vec<_>>>();

        if altitudes.is_none() && self.course.records.iter().any(|r| r.point.ele().is_some()) {
            warn!(
                "Some records lack an elevation the FIT altitude field can represent, so no \
                 altitudes will be written"
            );
        }
        altitudes
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
    fn get_data_size(&self, with_altitude: bool) -> usize {
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

        sz += CourseFile::get_definition_message_size(
            RecordMessage::field_definitions(with_altitude).len(),
        );
        sz += self.course.records.len()
            * CourseFile::get_data_message_size(RecordMessage::field_definitions(with_altitude));

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
    use dimensioned::si::M;

    use super::{CheckSummingWrite, Crc, FileHeader, FitAltitude, RecordMessage, Result};

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

    #[test]
    fn test_altitude_scale_and_offset() {
        // Sea level sits at the profile's offset, and each meter is five
        // increments of the encoded value.
        assert_eq!(FitAltitude::new(0.0 * M).unwrap().value_unsafe, 2500);
        assert_eq!(FitAltitude::new(93.0 * M).unwrap().value_unsafe, 2965);
        assert_eq!(FitAltitude::new(-100.0 * M).unwrap().value_unsafe, 2000);
    }

    #[test]
    fn test_altitude_rounds_to_nearest() {
        // The profile's resolution is 20cm, so intermediate elevations round to
        // the nearest representable value.
        assert_eq!(FitAltitude::new(1.1 * M).unwrap().value_unsafe, 2506);
        assert_eq!(FitAltitude::new(1.09 * M).unwrap().value_unsafe, 2505);
    }

    #[test]
    fn test_altitude_limits() {
        // The extremes of what the field can represent...
        assert_eq!(FitAltitude::new(-500.0 * M).unwrap().value_unsafe, 0);
        assert_eq!(FitAltitude::new(12606.8 * M).unwrap().value_unsafe, 65534);

        // ...beyond which there's no altitude to write, rather than one that
        // has wrapped around or aliased the profile's invalid value.
        assert_eq!(FitAltitude::new(-500.2 * M), None);
        assert_eq!(FitAltitude::new(12607.0 * M), None);
        assert_eq!(FitAltitude::new(f64::NAN * M), None);
    }

    #[test]
    fn test_record_altitude_field_definition() {
        // The altitude field is only defined for courses that have elevation
        // data, so that files without it are unchanged.
        assert_eq!(RecordMessage::field_definitions(false).len(), 4);

        let with_altitude = RecordMessage::field_definitions(true);
        assert_eq!(with_altitude.len(), 5);
        let altitude = with_altitude.last().unwrap();
        assert_eq!(altitude.field_number, 2);
        assert_eq!(altitude.size, 2);
        assert_eq!(altitude.base_type, 132);
    }
}
