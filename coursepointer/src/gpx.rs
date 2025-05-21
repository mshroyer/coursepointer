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
}

impl<R> GpxTrackReader<R>
where
    R: BufRead,
{
    pub fn new(reader: Reader<R>) -> GpxTrackReader<R> {
        Self { reader }
    }
}

/// An elevation from sea level, measured in meters.
#[derive(Copy, Clone, Debug)]
pub struct Elevation {
    meters: f64,
}

#[derive(Clone, Debug)]
pub enum GpxTrackElement {
    Name(String),
    Trackpoint(SurfacePoint, Option<Elevation>),
    Waypoint(SurfacePoint, String),
}

type EltPath = Vec<Vec<u8>>;

impl<R> Iterator for GpxTrackReader<R>
where
    R: BufRead,
{
    type Item = Result<GpxTrackElement>;

    fn next(&mut self) -> Option<Result<GpxTrackElement>> {
        let mut buf = Vec::new();
        let mut path: EltPath = Vec::new();
        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;

        loop {
            match self.reader.read_event_into(&mut buf) {
                Err(err) => return Some(Err(GpxError::Xml(err))),

                Ok(Event::Eof) => return None,

                Ok(Event::Start(elt)) => {
                    let name_bytes = elt.name().as_ref().to_owned();
                    path.push(name_bytes);
                    match elt.name().as_ref() {
                        b"trkpt" => {
                            lat = None;
                            lon = None;
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

                Ok(Event::Text(text)) => {
                    let trk_name_path: Vec<Vec<u8>> =
                        vec![b"gpx".into(), b"trk".into(), b"name".into()];
                    if path == trk_name_path {
                        return Some(match str::from_utf8(text.as_ref()) {
                            Err(err) => Err(GpxError::Utf8Error(err)),
                            Ok(s) => Ok(GpxTrackElement::Name(s.to_owned())),
                        });
                    } else {
                        continue;
                    }
                }

                Ok(Event::End(elt)) => {
                    // For consistency, keep the current element's name on the
                    // path until we're done with the End event.
                    EltPathPopper::new(&mut path);

                    match elt.name().as_ref() {
                        b"trkpt" => {
                            return Some(match (lat, lon) {
                                (Some(lat_val), Some(lon_val)) => {
                                    Ok(GpxTrackElement::Trackpoint(
                                        SurfacePoint::new(lat_val, lon_val),
                                        None,
                                    ))
                                },
                                _ => Err(GpxError::GpxSchemaError(
                                    "trkpt element missing lat or lon".to_owned(),
                                )),
                            })
                        },

                        _ => (),
                    }
                }

                _ => (),
            }
        }
    }
}

struct EltPathPopper<'a> {
    path: &'a mut EltPath,
}

impl <'a> EltPathPopper<'a> {
    fn new(path: &'a mut EltPath) -> Self {
        Self { path }
    }
}

impl Drop for EltPathPopper<'_> {
    fn drop(&mut self) {
        self.path.pop();
    }
}

#[cfg(test)]
mod tests {
    use geographic::SurfacePoint;
    use quick_xml::Reader;

    use super::Result;
    use super::{GpxTrackElement, GpxTrackReader};

    macro_rules! surface_points {
        ( $( ( $lat:expr, $lon:expr ) ),* $(,)? ) => {
            vec![ $( SurfacePoint::new($lat, $lon) ),* ]
        };
    }

    #[test]
    fn test_trackpoints() -> Result<()> {
        let xml = r#"
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
"#;

        let expected = surface_points![
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
                GpxTrackElement::Trackpoint(p, _) => Some(*p),
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
                GpxTrackElement::Name(n) => Some(n),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(result, vec!["Coyote"]);
        Ok(())
    }
}
