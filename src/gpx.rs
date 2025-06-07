//! GPX track/route and waypoint reader
//!
//! # Usage
//!
//! Provides an iterator that reads in sequence the trackpoints/routepoints,
//! waypoints, and other relevant items from a GPX track file.
//!
//! This module treats GPX routes and tracks synonymously, except that tracks
//! may also contain segments.
//!
//! To use this module, instantiate a [`GpxReader`] by calling
//! [`GpxReader::from_reader`]. Iterating over the [`GpxReader`] will produce a
//! sequence of [`GpxItem`] describing the contents of the input.

use std::io::BufRead;
use std::num::ParseFloatError;
use std::{mem, str};

use dimensioned::si::{M, Meter};
use quick_xml::events::Event;
use quick_xml::events::attributes::AttrError;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use thiserror::Error;
use tracing::debug;

use crate::measure::{DEG, Degree};
use crate::types::{GeoPoint, TypeError};

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
#[derive(Clone, PartialEq, Debug)]
pub enum GpxItem {
    /// Indicates the start of a GPX track or route.  Subsequent
    /// `TrackOrRouteName`, `TrackSegment`, and `TrackOrRoutePoint` items belong
    /// to this track or route.
    TrackOrRoute,
    /// Optionally provides the name of a GPX track or route.
    TrackOrRouteName(String),
    /// Indicates the start of a GPX track segment.  Subsequent
    /// `TrackOrRoutePoint` items belong to this segment, until the next
    /// `TrackOrRoute` or `TrackSegment` is encountered.
    TrackSegment,
    /// A point along a track segment or a route, returned in order of its
    /// position along the track or route.
    TrackOrRoutePoint(GeoPoint),
    /// A waypoint.  Global to the GPX document; not specifically associated
    /// with any track or route.
    Waypoint(Waypoint),
}

impl TryFrom<NextPtFields> for GeoPoint {
    type Error = GpxError;

    fn try_from(mut value: NextPtFields) -> Result<Self> {
        let lat = value.lat.take().ok_or(GpxError::GpxSchema(
            "trackpoint missing lat attribute".to_owned(),
        ))?;
        let lon = value.lon.take().ok_or(GpxError::GpxSchema(
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

    /// Comment, if specified.
    pub cmt: Option<String>,

    /// Symbol, if specified.
    pub sym: Option<String>,

    /// Waypoint type, if specified.
    pub type_: Option<String>,

    /// Position of the waypoint.
    pub point: GeoPoint,
}

impl TryFrom<NextPtFields> for Waypoint {
    type Error = GpxError;

    fn try_from(value: NextPtFields) -> Result<Self> {
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
            .ok_or(GpxError::GpxSchema("waypoint missing name".to_owned()))?;

        Ok(Self {
            name,
            cmt: value.cmt,
            sym: value.sym,
            type_: value.type_,
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
    num_tag_start: usize,
    num_tag_end: usize,
    num_next: usize,
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
            next_pt_fields: NextPtFields::default(),
            num_tag_start: 0,
            num_tag_end: 0,
            num_next: 0,
        }
    }
}

impl<R: BufRead> GpxReader<R> {
    pub fn from_reader(reader: R) -> GpxReader<R> {
        GpxReader::new(Reader::from_reader(reader))
    }
}

#[derive(Default)]
struct NextPtFields {
    name: Option<String>,
    cmt: Option<String>,
    sym: Option<String>,
    type_: Option<String>,
    lat: Option<Degree<f64>>,
    lon: Option<Degree<f64>>,
    ele: Option<Meter<f64>>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Tag {
    Gpx,
    Trk,
    Name,
    Trkseg,
    Trkpt,
    Rte,
    Rtept,
    Ele,
    Wpt,
    Cmt,
    Sym,
    Type,
    Unknown,
}

fn get_tag(name: &[u8]) -> Tag {
    match name {
        b"gpx" => Tag::Gpx,
        b"trk" => Tag::Trk,
        b"trkseg" => Tag::Trkseg,
        b"trkpt" => Tag::Trkpt,
        b"rte" => Tag::Rte,
        b"rtept" => Tag::Rtept,
        b"ele" => Tag::Ele,
        b"name" => Tag::Name,
        b"wpt" => Tag::Wpt,
        b"cmt" => Tag::Cmt,
        b"sym" => Tag::Sym,
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
        self.num_next += 1;
        let mut buf = Vec::new();

        // Keep iterating through quick_xml events until a new GpxItem can be
        // successfully emitted, any error occurs, or EOF is reached.
        loop {
            match self.reader.read_event_into(&mut buf) {
                Err(err) => return Some(Err(GpxError::Xml(err))),

                Ok(Event::Eof) => {
                    debug!(
                        "GpxReader processed {} tag start and {} tag end events in {} iterations",
                        self.num_tag_start, self.num_tag_end, self.num_next
                    );
                    return None;
                }

                Ok(Event::Start(elt)) => {
                    self.num_tag_start += 1;
                    let tag = get_tag(elt.name().as_ref());
                    self.tag_path.push(tag);

                    match self.tag_path.as_slice() {
                        [Tag::Gpx, Tag::Trk] | [Tag::Gpx, Tag::Rte] => {
                            debug!("Found start of track or route at path: {:?}", self.tag_path);
                            return Some(Ok(GpxItem::TrackOrRoute));
                        }

                        [Tag::Gpx, Tag::Trk, Tag::Trkseg] => {
                            return Some(Ok(GpxItem::TrackSegment));
                        }

                        [Tag::Gpx, Tag::Trk, Tag::Trkseg, Tag::Trkpt]
                        | [Tag::Gpx, Tag::Rte, Tag::Rtept]
                        | [Tag::Gpx, Tag::Wpt] => {
                            if let Err(e) = (|| {
                                for attr in elt.attributes() {
                                    let a = attr?;
                                    if a.key == QName(b"lat") {
                                        self.next_pt_fields.lat =
                                            Some(str::from_utf8(&a.value)?.parse::<f64>()? * DEG);
                                    } else if a.key == QName(b"lon") {
                                        self.next_pt_fields.lon =
                                            Some(str::from_utf8(&a.value)?.parse::<f64>()? * DEG);
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
                    [Tag::Gpx, Tag::Trk, Tag::Name] | [Tag::Gpx, Tag::Rte, Tag::Name] => {
                        return Some(match str::from_utf8(text.as_ref()) {
                            Err(err) => Err(GpxError::Utf8(err)),
                            Ok(s) => Ok(GpxItem::TrackOrRouteName(s.to_owned())),
                        });
                    }

                    [Tag::Gpx, Tag::Trk, Tag::Trkseg, Tag::Trkpt, Tag::Ele]
                    | [Tag::Gpx, Tag::Rte, Tag::Rtept, Tag::Ele]
                    | [Tag::Gpx, Tag::Wpt, Tag::Ele] => {
                        if let Err(e) = (|| {
                            self.next_pt_fields.ele =
                                Some(str::from_utf8(text.as_ref())?.parse::<f64>()? * M);
                            Ok(())
                        })() {
                            return Some(Err(e));
                        }
                    }

                    [Tag::Gpx, Tag::Wpt, Tag::Name] => match str::from_utf8(text.as_ref()) {
                        Ok(name) => self.next_pt_fields.name = Some(name.to_owned()),
                        Err(err) => return Some(Err(err.into())),
                    },

                    [Tag::Gpx, Tag::Wpt, Tag::Cmt] => match str::from_utf8(text.as_ref()) {
                        Ok(type_) => self.next_pt_fields.cmt = Some(type_.to_owned()),
                        Err(err) => return Some(Err(err.into())),
                    },

                    [Tag::Gpx, Tag::Wpt, Tag::Sym] => match str::from_utf8(text.as_ref()) {
                        Ok(type_) => self.next_pt_fields.sym = Some(type_.to_owned()),
                        Err(err) => return Some(Err(err.into())),
                    },

                    [Tag::Gpx, Tag::Wpt, Tag::Type] => match str::from_utf8(text.as_ref()) {
                        Ok(type_) => self.next_pt_fields.type_ = Some(type_.to_owned()),
                        Err(err) => return Some(Err(err.into())),
                    },

                    _ => (),
                },

                Ok(Event::End(_elt)) => {
                    self.num_tag_end += 1;
                    let tag_path = self.tag_path.clone();
                    self.tag_path.pop();

                    match tag_path.as_slice() {
                        [Tag::Gpx, Tag::Trk, Tag::Trkseg, Tag::Trkpt]
                        | [Tag::Gpx, Tag::Rte, Tag::Rtept] => {
                            return Some(
                                match GeoPoint::try_from(mem::take(&mut self.next_pt_fields)) {
                                    Ok(p) => Ok(GpxItem::TrackOrRoutePoint(p)),
                                    Err(e) => Err(e),
                                },
                            );
                        }

                        [Tag::Gpx, Tag::Wpt] => {
                            debug!("Found waypoint with name: {:?}", self.next_pt_fields.name);
                            return Some(
                                match Waypoint::try_from(mem::take(&mut self.next_pt_fields)) {
                                    Ok(p) => Ok(GpxItem::Waypoint(p)),
                                    Err(e) => Err(e),
                                },
                            );
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
    use quick_xml::Reader;

    use super::{GpxError, GpxItem, GpxReader, Result, Waypoint};
    use crate::geo_points;
    use crate::measure::DEG;
    use crate::types::GeoPoint;

    macro_rules! waypoint {
        ( $name:expr, $cmt:expr, $sym:expr, $type_:expr, $lat:expr, $lon:expr ) => {
            Waypoint {
                name: $name.to_owned(),
                cmt: $cmt.map(|s| s.to_owned()),
                sym: $sym.map(|s| s.to_owned()),
                type_: $type_.map(|s| s.to_owned()),
                point: GeoPoint::new($lat * DEG, $lon * DEG, None)?,
            }
        };
        ( $name:expr, $cmt:expr, $sym:expr, $type_:expr, $lat:expr, $lon:expr, $ele:expr ) => {
            Waypoint {
                name: $name.to_owned(),
                cmt: $cmt.map(|s| s.to_owned()),
                sym: $sym.map(|s| s.to_owned()),
                type_: $type_.map(|s| s.to_owned()),
                point: GeoPoint::new($lat * DEG, $lon * DEG, Some($ele * M))?,
            }
        };
    }

    macro_rules! waypoints {
        ( $( ( $name:expr, $cmt:expr, $sym:expr, $type_:expr, $lat:expr, $lon:expr $( , $ele:expr )? $(,)? ) ),* $(,)? ) => {
            vec![ $( waypoint!($name, $cmt, $sym, $type_, $lat, $lon $( , $ele )? ) ),* ]
        };
    }

    impl GpxReader<&[u8]> {
        pub fn from_text(s: &str) -> GpxReader<&[u8]> {
            GpxReader::new(Reader::from_reader(s.as_bytes()))
        }
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

        let reader = GpxReader::from_text(xml);
        let items = reader.collect::<Result<Vec<_>>>()?;
        let result = items
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackOrRoutePoint(p) => Some(*p),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_routepoints() -> Result<()> {
        let xml = r#"
<gpx>
  <rte>
    <name>Coyote</name>
    <rtept lat="37.39987" lon="-122.13737" />
    <rtept lat="37.39958" lon="-122.13684" />
    <rtept lat="37.39923" lon="-122.13591" />
    <rtept lat="37.39888" lon="-122.13498" />
  </rte>
</gpx>
"#;

        let expected = geo_points![
            (37.39987, -122.13737),
            (37.39958, -122.13684),
            (37.39923, -122.13591),
            (37.39888, -122.13498),
        ];

        let reader = GpxReader::from_text(xml);
        let items = reader.collect::<Result<Vec<_>>>()?;
        let result = items
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackOrRoutePoint(p) => Some(*p),
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

        let reader = GpxReader::from_text(xml);
        let items = reader.collect::<Result<Vec<_>>>()?;
        let result = items
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackOrRoutePoint(p) => Some(*p),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_routepoints_with_elevation() -> Result<()> {
        let xml = r#"
<gpx>
  <rte>
    <name>Coyote</name>
    <rtept lat="37.39987" lon="-122.13737">
      <ele>30.5</ele>
    </rtept>
    <rtept lat="37.39958" lon="-122.13684">
      <ele>29.9</ele>
    </rtept>
    <rtept lat="37.39923" lon="-122.13591">
      <ele>29.8</ele>
    </rtept>
    <rtept lat="37.39888" lon="-122.13498">
      <ele>31.8</ele>
    </rtept>
  </rte>
</gpx>
"#;

        let expected = geo_points![
            (37.39987, -122.13737, 30.5),
            (37.39958, -122.13684, 29.9),
            (37.39923, -122.13591, 29.8),
            (37.39888, -122.13498, 31.8),
        ];

        let reader = GpxReader::from_text(xml);
        let items = reader.collect::<Result<Vec<_>>>()?;
        let result = items
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackOrRoutePoint(p) => Some(*p),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_invalid_trackpoint() -> Result<()> {
        let xml = r#"
<gpx>
  <trk>
    <name>Foo</name>
    <trkseg>
      <trkpt lat="37.39987" lon="-122.13737">
        <ele>30.5</ele>
      </trkpt>
      <trkpt lat="37.39958">
        <ele>29.9</ele>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
"#;

        let reader = GpxReader::from_text(xml);
        let result = reader.collect::<Result<Vec<_>>>();
        assert!(
            matches!(result, Err(GpxError::GpxSchema(mesg)) if mesg == "trackpoint missing lon attribute".to_owned())
        );

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

        let reader = GpxReader::from_text(xml);
        let items = reader.collect::<Result<Vec<_>>>()?;
        let result = items
            .iter()
            .filter_map(|ele| match ele {
                GpxItem::TrackOrRouteName(n) => Some(n),
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

        let reader = GpxReader::from_text(xml);
        let items = reader.collect::<Result<Vec<_>>>()?;

        assert_eq!(
            items
                .iter()
                .filter(|e| match e {
                    GpxItem::TrackOrRoute => true,
                    _ => false,
                })
                .collect::<Vec<_>>()
                .len(),
            1
        );

        assert_eq!(
            items
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
                Some("trailhead"),
                Some("Dot"),
                Some("info"),
                37.40147999999951,
                -122.12117999999951,
            ),
            (
                "Trail Turn-off",
                Some("trailhead"),
                Some("Dot"),
                Some("info"),
                37.39866999999887,
                -122.13531999999954,
            ),
            (
                "Trail ends",
                Some("generic"),
                Some("Dot"),
                Some("generic"),
                37.38693915264021,
                -122.15257150642014,
            ),
        ];

        let reader = GpxReader::from_text(xml);
        let items = reader.collect::<Result<Vec<_>>>()?;
        let result = items
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
