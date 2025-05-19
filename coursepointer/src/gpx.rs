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

pub struct GpxTrackReader<R>
where
    R: BufRead,
{
    reader: Reader<R>,
}

pub enum GpxTrackElement {
    Eof,
    Trackpoint(SurfacePoint),
}

impl<R> GpxTrackReader<R>
where
    R: BufRead,
{
    pub fn new(reader: Reader<R>) -> GpxTrackReader<R> {
        Self { reader }
    }

    pub fn next<C>(&mut self) -> Result<GpxTrackElement> {
        // let mut txt = Vec::new();
        let mut buf = Vec::new();

        loop {
            match self.reader.read_event_into(&mut buf) {
                Err(err) => return Err(GpxError::Xml(err)),

                Ok(Event::Eof) => return Ok(GpxTrackElement::Eof),

                Ok(Event::Start(ele)) => match ele.name().as_ref() {
                    b"trkpt" => {
                        let mut lat: Option<f64> = None;
                        let mut lon: Option<f64> = None;
                        for attr in ele.attributes() {
                            let a = attr?;
                            if a.key == QName(b"lat") {
                                lat = Some(str::from_utf8(&a.value)?.parse()?);
                            } else if a.key == QName(b"lon") {
                                lon = Some(str::from_utf8(&a.value)?.parse()?);
                            }
                        }
                        return match (lat, lon) {
                            (Some(lat), Some(lon)) => {
                                Ok(GpxTrackElement::Trackpoint(SurfacePoint::new(lat, lon)))
                            }
                            _ => Err(GpxError::GpxSchemaError(
                                "trkpt element missing lat or lon".to_owned(),
                            )),
                        };
                    }
                    
                    _ => (),
                },

                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Result;

    #[test]
    fn test_foo() -> Result<()> {
        Ok(())
    }
}
