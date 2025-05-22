use std::io::BufRead;
use std::num::ParseFloatError;
use std::str;

use quick_xml;
use quick_xml::events::Event;
use quick_xml::events::attributes::AttrError;
use quick_xml::name::QName;
use quick_xml::reader::Reader;
use thiserror::Error;

use geographic::SurfacePoint;

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

/// A reader for GPX Track files
///
/// Implements an Iterator that emits the track's trackpoints and waypoints.
pub struct GpxTrackReader<R>
where
    R: BufRead,
{
    reader: Reader<R>,
    tag_path: TagNamePath,
}

impl<R> GpxTrackReader<R>
where
    R: BufRead,
{
    pub fn new(mut reader: Reader<R>) -> GpxTrackReader<R> {
        // Needed because our parsing logic relies on maintaining a stack of tag
        // names, which would otherwise be broken by empty trkpt tags not
        // generating an "End" event.
        reader.config_mut().expand_empty_elements = true;

        Self {
            reader,
            tag_path: vec![],
        }
    }
}

/// An elevation from sea level, measured in meters.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Elevation {
    meters: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub enum GpxTrackItem {
    Name(String),
    Track,
    TrackSegment,
    TrackPoint(SurfacePoint, Option<Elevation>),
    Waypoint(SurfacePoint, String),
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum TagName {
    Gpx,
    Trk,
    Trkseg,
    Trkpt,
    Ele,
    Name,
    Unknown,
}

fn get_name(name: &[u8]) -> TagName {
    match name {
        b"gpx" => TagName::Gpx,
        b"trk" => TagName::Trk,
        b"trkseg" => TagName::Trkseg,
        b"trkpt" => TagName::Trkpt,
        b"ele" => TagName::Ele,
        b"name" => TagName::Name,
        _ => TagName::Unknown,
    }
}

type TagNamePath = Vec<TagName>;

impl<R> Iterator for GpxTrackReader<R>
where
    R: BufRead,
{
    type Item = Result<GpxTrackItem>;

    fn next(&mut self) -> Option<Result<GpxTrackItem>> {
        let mut buf = Vec::new();
        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;
        let mut ele: Option<Elevation> = None;

        loop {
            match self.reader.read_event_into(&mut buf) {
                Err(err) => return Some(Err(GpxError::Xml(err))),

                Ok(Event::Eof) => return None,

                Ok(Event::Start(elt)) => {
                    let name = get_name(elt.name().as_ref());
                    self.tag_path.push(name);

                    match self.tag_path.as_slice() {
                        [TagName::Gpx, TagName::Trk] => {
                            return Some(Ok(GpxTrackItem::Track));
                        }

                        [TagName::Gpx, TagName::Trk, TagName::Trkseg] => {
                            return Some(Ok(GpxTrackItem::TrackSegment));
                        }

                        [TagName::Gpx, TagName::Trk, TagName::Trkseg, TagName::Trkpt] => {
                            lat = None;
                            lon = None;
                            ele = None;
                            if let Err(e) = (|| {
                                for attr in elt.attributes() {
                                    let a = attr?;
                                    if a.key == QName(b"lat") {
                                        lat = Some(str::from_utf8(&a.value)?.parse()?);
                                    } else if a.key == QName(b"lon") {
                                        lon = Some(str::from_utf8(&a.value)?.parse()?);
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
                    [TagName::Gpx, TagName::Trk, TagName::Name] => {
                        return Some(match str::from_utf8(text.as_ref()) {
                            Err(err) => Err(GpxError::Utf8Error(err)),
                            Ok(s) => Ok(GpxTrackItem::Name(s.to_owned())),
                        });
                    }

                    [
                        TagName::Gpx,
                        TagName::Trk,
                        TagName::Trkseg,
                        TagName::Trkpt,
                        TagName::Ele,
                    ] => {
                        if let Err(e) = (|| {
                            ele = Some(Elevation {
                                meters: str::from_utf8(text.as_ref())?.parse()?,
                            });
                            Ok(())
                        })() {
                            return Some(Err(e));
                        }
                    }

                    _ => {
                        continue;
                    }
                },

                Ok(Event::End(elt)) => {
                    // For consistency, keep the current element's name on the
                    // path until we're done with the End event.
                    TagNamePopper::new(&mut self.tag_path);

                    match get_name(elt.name().as_ref()) {
                        TagName::Trkpt => {
                            return Some(match (lat, lon) {
                                (Some(lat_val), Some(lon_val)) => Ok(GpxTrackItem::TrackPoint(
                                    SurfacePoint::new(lat_val, lon_val),
                                    ele,
                                )),
                                _ => Err(GpxError::GpxSchemaError(
                                    "trkpt element missing lat or lon".to_owned(),
                                )),
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

struct TagNamePopper<'a> {
    path: &'a mut TagNamePath,
}

impl<'a> TagNamePopper<'a> {
    fn new(path: &'a mut TagNamePath) -> Self {
        Self { path }
    }
}

impl Drop for TagNamePopper<'_> {
    fn drop(&mut self) {
        self.path.pop();
    }
}

#[cfg(test)]
mod tests {
    use geographic::SurfacePoint;
    use quick_xml::Reader;

    use super::Result;
    use super::{Elevation, GpxTrackItem, GpxTrackReader};

    macro_rules! track_point {
        ( $lat:expr, $lon:expr ) => {
            GpxTrackItem::TrackPoint(SurfacePoint::new($lat, $lon), None)
        };
        ( $lat:expr, $lon:expr, $ele:expr ) => {
            GpxTrackItem::TrackPoint(
                SurfacePoint::new($lat, $lon),
                Some(Elevation { meters: $ele }),
            )
        };
    }

    macro_rules! track_points {
        ( $( ( $lat:expr, $lon:expr $(, $ele:expr )? ) ),* $(,)? ) => {
            vec![ $( track_point!($lat, $lon $( , $ele )?) ),* ]
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

        let reader = GpxTrackReader::new(Reader::from_str(xml));
        let elements = reader.collect::<Result<Vec<_>>>()?;
        let result = elements
            .iter()
            .filter_map(|ele| match ele {
                GpxTrackItem::TrackPoint(p, ele) => Some(GpxTrackItem::TrackPoint(*p, *ele)),
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

        let mut reader = Reader::from_str(xml);
        reader.config_mut().expand_empty_elements = true;
        let track_reader = GpxTrackReader::new(reader);
        let elements = track_reader.collect::<Result<Vec<_>>>()?;
        let result = elements
            .iter()
            .filter_map(|ele| match ele {
                GpxTrackItem::TrackPoint(p, ele) => Some(GpxTrackItem::TrackPoint(*p, *ele)),
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

        let reader = GpxTrackReader::new(Reader::from_str(xml));
        let elements = reader.collect::<Result<Vec<_>>>()?;
        let result = elements
            .iter()
            .filter_map(|ele| match ele {
                GpxTrackItem::Name(n) => Some(n),
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

        let reader = GpxTrackReader::new(Reader::from_str(xml));
        let elements = reader.collect::<Result<Vec<_>>>()?;

        assert_eq!(elements.iter().filter(|e| match e {
            GpxTrackItem::Track => true,
            _ => false,
        }).collect::<Vec<_>>().len(), 1);

        assert_eq!(elements.iter().filter(|e| match e {
            GpxTrackItem::TrackSegment => true,
            _ => false,
        }).collect::<Vec<_>>().len(), 1);

        Ok(())
    }
}
