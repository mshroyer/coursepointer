//! GPX track and waypoint reader
//!
//! # Usage
//!
//! Provides an iterator that reads in sequence the trackpoints, waypoints,
//! and other relevant items from a GPX track file.
//!
//! To use this module, instantiate a [`GpxReader`] by invoking
//! [`GpxReader::from_str`] or [`GpxReader::from_path`]. Iterating over the
//! [`GpxReader`] will produce a sequence of [`GpxItem`] describing the contents
//! of the input.
//!
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::num::ParseFloatError;
use std::path::Path;
use std::str;

use quick_xml;
use quick_xml::events::Event;
use quick_xml::events::attributes::AttrError;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use thiserror::Error;

use crate::measure::Degrees;
use crate::measure::Meters;

/// An error processing a GPX track file.
#[derive(Error, Debug)]
pub enum GpxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("XML attribute error: {0}")]
    AttrError(#[from] AttrError),
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] str::Utf8Error),
    #[error("parsing floating-point number: {0}")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("GPX schema error: {0}")]
    GpxSchemaError(String),
}

type Result<T> = std::result::Result<T, GpxError>;

/// An item parsed from a GPX document.
#[derive(Clone, PartialEq, Debug)]
pub enum GpxItem {
    /// Indicates the start of a GPX track.  Subsequent TrackName, TrackSegment,
    /// and TrackPoint items belong to this track.
    Track,
    /// Optionally provides the name of a GPX track.
    TrackName(String),
    /// Indicates the start of a GPX track segment.  Subsequent TrackPoints
    /// belong to this segment.
    TrackSegment,
    /// A point along a track segment, returned in order of its position along
    /// the track.
    TrackPoint(TrackPoint),
    /// A waypoint.  Global to the GPX document; not specifically associated
    /// with any track.
    Waypoint(Waypoint),
}

/// A GPX trackpoint or route point.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TrackPoint {
    /// Latitude in decimal degrees.
    pub lat: Degrees<f64>,

    /// Longitude in decimal degrees.
    pub lon: Degrees<f64>,

    /// Elevation in meters, if known.
    pub ele: Option<Meters<f64>>,
}

impl TryFrom<&NextPtFields> for TrackPoint {
    type Error = GpxError;

    fn try_from(value: &NextPtFields) -> Result<Self> {
        let lat = value.lat.ok_or(GpxError::GpxSchemaError(
            "trackpoint missing lat attribute".to_owned(),
        ))?;
        let lon = value.lon.ok_or(GpxError::GpxSchemaError(
            "trackpoint missing lon attribute".to_owned(),
        ))?;
        Ok(Self {
            lat,
            lon,
            ele: value.ele,
        })
    }
}

/// A GPX waypoint.
#[derive(Clone, PartialEq, Debug)]
pub struct Waypoint {
    /// Latitude in decimal degrees.
    pub lat: Degrees<f64>,

    /// Longitude in decimal degrees.
    pub lon: Degrees<f64>,

    /// Elevation in meters, if known.
    pub ele: Option<Meters<f64>>,

    /// Waypoint name.
    pub name: String,

    /// Waypoint type, if specified.
    pub type_: Option<String>,
}

impl TryFrom<&NextPtFields> for Waypoint {
    type Error = GpxError;

    fn try_from(value: &NextPtFields) -> Result<Self> {
        let lat = value.lat.ok_or(GpxError::GpxSchemaError(
            "waypoint missing lat attribute".to_owned(),
        ))?;
        let lon = value.lon.ok_or(GpxError::GpxSchemaError(
            "waypoint missing lon attribute".to_owned(),
        ))?;

        // The GPX 1.1 schema doesn't strictly require waypoints to have a name,
        // but for our purposes this is a requirement.
        let name = value
            .name
            .clone()
            .ok_or(GpxError::GpxSchemaError("waypoint missing name".to_owned()))?;

        Ok(Self {
            lat,
            lon,
            ele: value.ele,
            name,
            type_: value.type_.clone(),
        })
    }
}

/// A reader for GPX Track files
///
/// Implements an Iterator that emits the track's trackpoints and waypoints.
pub struct GpxReader<R>
where
    R: BufRead,
{
    reader: Reader<R>,
    tag_path: TagPath,
    next_pt_fields: NextPtFields,
}

impl<R> GpxReader<R>
where
    R: BufRead,
{
    fn new(mut reader: Reader<R>) -> GpxReader<R> {
        // Needed because our parsing logic relies on maintaining a stack of tag
        // names, which would otherwise be broken by empty trkpt tags not
        // generating an "End" event.
        reader.config_mut().expand_empty_elements = true;

        Self {
            reader,
            tag_path: vec![],
            next_pt_fields: NextPtFields::new(),
        }
    }
}

impl GpxReader<&[u8]> {
    pub fn from_str(s: &str) -> GpxReader<&[u8]> {
        GpxReader::new(Reader::from_reader(s.as_bytes()))
    }
}

impl GpxReader<BufReader<File>> {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<GpxReader<BufReader<File>>> {
        Ok(GpxReader::new(Reader::from_file(path)?))
    }
}

struct NextPtFields {
    lat: Option<Degrees<f64>>,
    lon: Option<Degrees<f64>>,
    ele: Option<Meters<f64>>,
    name: Option<String>,
    type_: Option<String>,
}

impl NextPtFields {
    fn new() -> Self {
        Self {
            lat: None,
            lon: None,
            ele: None,
            name: None,
            type_: None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Tag {
    Gpx,
    Trk,
    Name,
    Trkseg,
    Trkpt,
    Ele,
    Wpt,
    Type,
    Unknown,
}

fn get_tag(name: &[u8]) -> Tag {
    match name {
        b"gpx" => Tag::Gpx,
        b"trk" => Tag::Trk,
        b"trkseg" => Tag::Trkseg,
        b"trkpt" => Tag::Trkpt,
        b"ele" => Tag::Ele,
        b"name" => Tag::Name,
        b"wpt" => Tag::Wpt,
        b"type" => Tag::Type,
        _ => Tag::Unknown,
    }
}

type TagPath = Vec<Tag>;

impl<R> Iterator for GpxReader<R>
where
    R: BufRead,
{
    type Item = Result<GpxItem>;

    fn next(&mut self) -> Option<Result<GpxItem>> {
        let mut buf = Vec::new();

        // Keep iterating through quick_xml events until a new GpxItem can be
        // successfully emitted, any error occurs, or EOF is reached.
        loop {
            match self.reader.read_event_into(&mut buf) {
                Err(err) => return Some(Err(GpxError::Xml(err))),

                Ok(Event::Eof) => return None,

                Ok(Event::Start(elt)) => {
                    let name = get_tag(elt.name().as_ref());
                    self.tag_path.push(name);

                    match self.tag_path.as_slice() {
                        [Tag::Gpx, Tag::Trk] => {
                            return Some(Ok(GpxItem::Track));
                        }

                        [Tag::Gpx, Tag::Trk, Tag::Trkseg] => {
                            return Some(Ok(GpxItem::TrackSegment));
                        }

                        [Tag::Gpx, Tag::Trk, Tag::Trkseg, Tag::Trkpt] | [Tag::Gpx, Tag::Wpt] => {
                            if let Err(e) = (|| {
                                self.next_pt_fields = NextPtFields::new();
                                for attr in elt.attributes() {
                                    let a = attr?;
                                    if a.key == QName(b"lat") {
                                        self.next_pt_fields.lat =
                                            Some(Degrees(str::from_utf8(&a.value)?.parse()?));
                                    } else if a.key == QName(b"lon") {
                                        self.next_pt_fields.lon =
                                            Some(Degrees(str::from_utf8(&a.value)?.parse()?));
                                    }
                                }
                                Ok(())
                            })() {
                                return Some(Err(e));
                            }
                        }

                        _ => (),
                    }
                }

                Ok(Event::Text(text)) => match self.tag_path.as_slice() {
                    [Tag::Gpx, Tag::Trk, Tag::Name] => {
                        return Some(match str::from_utf8(text.as_ref()) {
                            Err(err) => Err(GpxError::Utf8Error(err)),
                            Ok(s) => Ok(GpxItem::TrackName(s.to_owned())),
                        });
                    }

                    [Tag::Gpx, Tag::Trk, Tag::Trkseg, Tag::Trkpt, Tag::Ele]
                    | [Tag::Gpx, Tag::Wpt, Tag::Ele] => {
                        if let Err(e) = (|| {
                            self.next_pt_fields.ele =
                                Some(Meters(str::from_utf8(text.as_ref())?.parse()?));
                            Ok(())
                        })() {
                            return Some(Err(e));
                        }
                    }

                    [Tag::Gpx, Tag::Wpt, Tag::Name] => match str::from_utf8(text.as_ref()) {
                        Ok(name) => self.next_pt_fields.name = Some(name.to_owned()),
                        Err(err) => return Some(Err(err.into())),
                    },

                    [Tag::Gpx, Tag::Wpt, Tag::Type] => match str::from_utf8(text.as_ref()) {
                        Ok(type_) => self.next_pt_fields.type_ = Some(type_.to_owned()),
                        Err(err) => return Some(Err(err.into())),
                    },

                    _ => (),
                },

                Ok(Event::End(_elt)) => {
                    let tag_path = self.tag_path.clone();
                    self.tag_path.pop();

                    match tag_path.as_slice() {
                        [Tag::Gpx, Tag::Trk, Tag::Trkseg, Tag::Trkpt] => {
                            return Some(match TrackPoint::try_from(&self.next_pt_fields) {
                                Ok(p) => Ok(GpxItem::TrackPoint(p)),
                                Err(e) => Err(e),
                            });
                        }

                        [Tag::Gpx, Tag::Wpt] => {
                            return Some(match Waypoint::try_from(&self.next_pt_fields) {
                                Ok(p) => Ok(GpxItem::Waypoint(p)),
                                Err(e) => Err(e),
                            });
                        }

                        _ => (),
                    }
                }

                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::measure::Degrees;
    use crate::measure::Meters;

    use super::{GpxItem, GpxReader, Result, TrackPoint, Waypoint};

    macro_rules! track_point {
        ( $lat:expr, $lon:expr ) => {
            TrackPoint {
                lat: Degrees($lat),
                lon: Degrees($lon),
                ele: None,
            }
        };
        ( $lat:expr, $lon:expr, $ele:expr ) => {
            TrackPoint {
                lat: Degrees($lat),
                lon: Degrees($lon),
                ele: Some(Meters($ele)),
            }
        };
    }

    macro_rules! track_points {
        ( $( ( $lat:expr, $lon:expr $(, $ele:expr )? ) ),* $(,)? ) => {
            vec![ $( track_point!($lat, $lon $( , $ele )?) ),* ]
        };
    }

    macro_rules! waypoints {
        ( $( ( $lat:expr, $lon:expr, $ele:expr, $name:expr, $type_:expr $(,)? ) ),* $(,)? ) => {
            vec![ $( Waypoint {
                lat: Degrees($lat),
                lon: Degrees($lon),
                ele: $ele.map(Meters),
                name: $name.to_owned(),
                type_: $type_.map(|s| s.to_owned())
            } ),* ]
        };
    }

    #[test]
    fn test_trackpoints() -> Result<()> {
        let xml = r#"
<gpx>
  <trk>
    <name>Coyote</name>
    <trkseg>
      <trkpt lat="37.39987" lon="-122.13737" />
      <trkpt lat="37.39958" lon="-122.13684" />
      <trkpt lat="37.39923" lon="-122.13591" />
      <trkpt lat="37.39888" lon="-122.13498" />
    </trkseg>
  </trk>
</gpx>
"#;

        let expected = track_points![
            (37.39987, -122.13737),
            (37.39958, -122.13684),
            (37.39923, -122.13591),
            (37.39888, -122.13498),
        ];

        let reader = GpxReader::from_str(xml);
        let elements = reader.collect::<Result<Vec<_>>>()?;
        let result = elements
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackPoint(p) => Some(*p),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_trackpoints_with_elevation() -> Result<()> {
        let xml = r#"
<gpx>
  <trk>
    <name>Coyote</name>
    <trkseg>
      <trkpt lat="37.39987" lon="-122.13737">
        <ele>30.5</ele>
      </trkpt>
      <trkpt lat="37.39958" lon="-122.13684">
        <ele>29.9</ele>
      </trkpt>
      <trkpt lat="37.39923" lon="-122.13591">
        <ele>29.8</ele>
      </trkpt>
      <trkpt lat="37.39888" lon="-122.13498">
        <ele>31.8</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
"#;

        let expected = track_points![
            (37.39987, -122.13737, 30.5),
            (37.39958, -122.13684, 29.9),
            (37.39923, -122.13591, 29.8),
            (37.39888, -122.13498, 31.8),
        ];

        let reader = GpxReader::from_str(xml);
        let elements = reader.collect::<Result<Vec<_>>>()?;
        let result = elements
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackPoint(p) => Some(*p),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_track_name() -> Result<()> {
        let xml = r#"
<gpx>
  <trk>
    <name>Coyote</name>
    <trkseg>
      <trkpt lat="37.39987" lon="-122.13737">
        <ele>30.5</ele>
      </trkpt>
      <trkpt lat="37.39958" lon="-122.13684">
        <ele>29.9</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
"#;

        let reader = GpxReader::from_str(xml);
        let elements = reader.collect::<Result<Vec<_>>>()?;
        let result = elements
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackName(n) => Some(n),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, vec!["Coyote"]);
        Ok(())
    }

    #[test]
    fn test_track_and_track_segment() -> Result<()> {
        let xml = r#"
<gpx>
  <trk>
    <name>Coyote</name>
    <trkseg>
      <trkpt lat="37.39987" lon="-122.13737">
        <ele>30.5</ele>
      </trkpt>
      <trkpt lat="37.39958" lon="-122.13684">
        <ele>29.9</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
"#;

        let reader = GpxReader::from_str(xml);
        let elements = reader.collect::<Result<Vec<_>>>()?;

        assert_eq!(
            elements
                .iter()
                .filter(|e| match e {
                    GpxItem::Track => true,
                    _ => false,
                })
                .collect::<Vec<_>>()
                .len(),
            1
        );

        assert_eq!(
            elements
                .iter()
                .filter(|e| match e {
                    GpxItem::TrackSegment => true,
                    _ => false,
                })
                .collect::<Vec<_>>()
                .len(),
            1
        );

        Ok(())
    }

    #[test]
    fn test_waypoints() -> Result<()> {
        let xml = r#"
<gpx>
  <metadata>
    <name>TR017-Coyote</name>
    <link href="https://ridewithgps.com/routes/50344071">
      <text>TR017-Coyote</text>
    </link>
    <time>2025-04-15T16:09:37Z</time>
  </metadata>
  <wpt lat="37.40147999999951" lon="-122.12117999999951">
    <name>Hetch Hetchy Trail</name>
    <cmt>trailhead</cmt>
    <sym>Dot</sym>
    <type>info</type>
  </wpt>
  <wpt lat="37.39866999999887" lon="-122.13531999999954">
    <name>Trail Turn-off</name>
    <cmt>trailhead</cmt>
    <sym>Dot</sym>
    <type>info</type>
  </wpt>
  <wpt lat="37.38693915264021" lon="-122.15257150642014">
    <name>Trail ends</name>
    <cmt>generic</cmt>
    <sym>Dot</sym>
    <type>generic</type>
  </wpt>
  <trk>
    <name>Coyote</name>
    <trkseg>
      <trkpt lat="37.39987" lon="-122.13737">
        <ele>30.5</ele>
      </trkpt>
      <trkpt lat="37.39958" lon="-122.13684">
        <ele>29.9</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
"#;

        let expected = waypoints![
            (
                37.40147999999951,
                -122.12117999999951,
                None,
                "Hetch Hetchy Trail",
                Some("info"),
            ),
            (
                37.39866999999887,
                -122.13531999999954,
                None,
                "Trail Turn-off",
                Some("info"),
            ),
            (
                37.38693915264021,
                -122.15257150642014,
                None,
                "Trail ends",
                Some("generic"),
            ),
        ];

        let reader = GpxReader::from_str(xml);
        let elements = reader.collect::<Result<Vec<_>>>()?;
        let result = elements
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::Waypoint(p) => Some(p.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, expected);

        Ok(())
    }
}
