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

use coretypes::measure::Degrees;
use coretypes::measure::Meters;
use coretypes::{GeoPoint, TypeError};

/// An error processing a GPX track file.
#[derive(Error, Debug)]
pub enum GpxError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("XML processing error")]
    Xml(#[from] quick_xml::Error),
    #[error("XML attribute processing error")]
    XmlAttr(#[from] AttrError),
    #[error("UTF-8 decoding error")]
    Utf8(#[from] str::Utf8Error),
    #[error("parsing floating-point number")]
    ParseFloat(#[from] ParseFloatError),
    #[error("GPX schema error")]
    GpxSchema(String),
    #[error("type invariant error")]
    Type(#[from] TypeError),
}

type Result<T> = std::result::Result<T, GpxError>;

/// An item parsed from a GPX document.
///
/// TODO: Also support rte/rtept, as in Gaia GPS exports.
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
    TrackPoint(GeoPoint),
    /// A waypoint.  Global to the GPX document; not specifically associated
    /// with any track.
    Waypoint(Waypoint),
}

impl TryFrom<&NextPtFields> for GeoPoint {
    type Error = GpxError;

    fn try_from(value: &NextPtFields) -> Result<Self> {
        let lat = value.lat.ok_or(GpxError::GpxSchema(
            "trackpoint missing lat attribute".to_owned(),
        ))?;
        let lon = value.lon.ok_or(GpxError::GpxSchema(
            "trackpoint missing lon attribute".to_owned(),
        ))?;
        Ok(GeoPoint::new(lat, lon, value.ele)?)
    }
}

/// A GPX waypoint.
#[derive(Clone, PartialEq, Debug)]
pub struct Waypoint {
    /// Waypoint name.
    pub name: String,

    /// Waypoint type, if specified.
    pub type_: Option<String>,

    /// Position of the waypoint.
    pub point: GeoPoint,
}

impl TryFrom<&NextPtFields> for Waypoint {
    type Error = GpxError;

    fn try_from(value: &NextPtFields) -> Result<Self> {
        let lat = value.lat.ok_or(GpxError::GpxSchema(
            "waypoint missing lat attribute".to_owned(),
        ))?;
        let lon = value.lon.ok_or(GpxError::GpxSchema(
            "waypoint missing lon attribute".to_owned(),
        ))?;

        // The GPX 1.1 schema doesn't strictly require waypoints to have a name,
        // but for our purposes this is a requirement.
        let name = value
            .name
            .clone()
            .ok_or(GpxError::GpxSchema("waypoint missing name".to_owned()))?;

        Ok(Self {
            name,
            type_: value.type_.clone(),
            point: GeoPoint::new(lat, lon, None)?,
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
        let file = File::open(path)?;
        let buf_reader = BufReader::new(file);
        Ok(GpxReader::new(Reader::from_reader(buf_reader)))
    }
}

struct NextPtFields {
    name: Option<String>,
    type_: Option<String>,
    lat: Option<Degrees<f64>>,
    lon: Option<Degrees<f64>>,
    ele: Option<Meters<f64>>,
}

impl NextPtFields {
    fn new() -> Self {
        Self {
            name: None,
            type_: None,
            lat: None,
            lon: None,
            ele: None,
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
                            Err(err) => Err(GpxError::Utf8(err)),
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
                            return Some(match GeoPoint::try_from(&self.next_pt_fields) {
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
    use coretypes::GeoPoint;
    use coretypes::geo_points;
    use coretypes::measure::Degrees;
    use coretypes::measure::Meters;

    use super::{GpxItem, GpxReader, Result, Waypoint};

    macro_rules! waypoint {
        ( $name:expr, $type_:expr, $lat:expr, $lon:expr ) => {
            Waypoint {
                name: $name.to_owned(),
                type_: $type_.map(|s| s.to_owned()),
                point: GeoPoint::new(Degrees($lat), Degrees($lon), None)?,
            }
        };
        ( $name:expr, $type_:expr, $lat:expr, $lon:expr, $ele:expr ) => {
            Waypoint {
                name: $name.to_owned(),
                type_: $type_.map(|s| s.to_owned()),
                point: GeoPoint::new(Degrees($lat), Degrees($lon), Some(Meters($ele)))?,
            }
        };
    }

    macro_rules! waypoints {
        ( $( ( $name:expr, $type_:expr, $lat:expr, $lon:expr $( , $ele:expr )? $(,)? ) ),* $(,)? ) => {
            vec![ $( waypoint!($name, $type_, $lat, $lon $( , $ele )? ) ),* ]
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

        let expected = geo_points![
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

        let expected = geo_points![
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
                "Hetch Hetchy Trail",
                Some("info"),
                37.40147999999951,
                -122.12117999999951,
            ),
            (
                "Trail Turn-off",
                Some("info"),
                37.39866999999887,
                -122.13531999999954,
            ),
            (
                "Trail ends",
                Some("generic"),
                37.38693915264021,
                -122.15257150642014,
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
