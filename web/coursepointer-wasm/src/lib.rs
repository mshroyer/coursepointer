use std::io::Cursor;

use coursepointer::course::{Course, CoursePoint, CourseSetBuilder, CourseSetOptions, Record};
use coursepointer::internal::Kilometer;
use coursepointer::{
    CoursePointType, DEG, FitCourseOptions, GeoPoint, Sport, convert_gpx_to_fit, read_gpx,
};
use dimensioned::f64prefixes::KILO;
use dimensioned::si::{HR, M};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum WasmWrapperError {
    #[error("Error from CoursePointer library")]
    CoursePointer(#[from] coursepointer::CoursePointerError),
    #[error("Type invariant error")]
    TypeInvariant(#[from] coursepointer::TypeError),
    #[error("Error building course")]
    Course(#[from] coursepointer::course::CourseError),
    #[error("Serialization")]
    Serialization(#[from] serde_wasm_bindgen::Error),
    #[error("Miscellaneous error")]
    Anyhow(#[from] anyhow::Error),
}

impl From<WasmWrapperError> for JsValue {
    fn from(err: WasmWrapperError) -> JsValue {
        // JsValue::from_str(&err.to_string())
        JsValue::from_str(format!("{err:?}").as_str())
    }
}

pub type Result<T> = std::result::Result<T, WasmWrapperError>;

#[wasm_bindgen(start)]
pub fn init() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
}

#[derive(Serialize, Copy, Clone)]
#[wasm_bindgen]
pub struct JsGeoPoint {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub ele_m: f64,
}

impl From<GeoPoint> for JsGeoPoint {
    fn from(p: GeoPoint) -> Self {
        JsGeoPoint {
            lat_deg: p.lat().value_unsafe,
            lon_deg: p.lon().value_unsafe,
            ele_m: p.ele().unwrap_or(0.0 * M).value_unsafe,
        }
    }
}

#[derive(Copy, Clone)]
#[wasm_bindgen]
pub struct JsRecord {
    pub point: JsGeoPoint,
    pub cumulative_distance_m: f64,
}

impl From<Record> for JsRecord {
    fn from(r: Record) -> Self {
        JsRecord {
            point: r.point.into(),
            cumulative_distance_m: r.cumulative_distance.value_unsafe,
        }
    }
}

#[derive(Serialize, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsCoursePoint {
    pub point: JsGeoPoint,
    pub distance_m: f64,
    pub point_type: CoursePointType,
    pub name: String,
}

impl From<CoursePoint> for JsCoursePoint {
    fn from(cp: CoursePoint) -> Self {
        JsCoursePoint {
            point: cp.point.into(),
            distance_m: cp.distance.value_unsafe,
            point_type: cp.point_type,
            name: cp.name,
        }
    }
}

#[derive(Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsCourse {
    pub records: Vec<JsRecord>,
    pub course_points: Vec<JsCoursePoint>,
    pub name: String,
}

impl From<Course> for JsCourse {
    fn from(c: Course) -> Self {
        JsCourse {
            records: c.records.into_iter().map(Into::into).collect(),
            course_points: c.course_points.into_iter().map(Into::into).collect(),
            name: c.name.unwrap_or_default(),
        }
    }
}

#[wasm_bindgen]
pub fn read_gpx_bytes(data: &[u8]) -> Result<JsCourse> {
    let set = read_gpx(CourseSetOptions::default(), Cursor::new(data))?;
    Ok(set.courses[0].clone().into())
}

#[derive(Serialize, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsConversionInfo {
    pub course_name: String,
    pub total_distance_m: f64,
    pub num_waypoints: usize,
    pub course_points: Vec<JsCoursePoint>,
    pub fit_bytes: Box<[u8]>,
    pub report: String,
}

#[derive(Deserialize)]
pub struct JsOptions {
    pub sport: u8,
    pub speed: f64,
    pub threshold: f64,
}

impl From<&JsOptions> for FitCourseOptions {
    fn from(js: &JsOptions) -> Self {
        FitCourseOptions::default()
            .with_sport(Sport::try_from_primitive(js.sport).unwrap_or_default())
            .with_speed(js.speed * KILO * M / HR)
    }
}

impl From<&JsOptions> for CourseSetOptions {
    fn from(js: &JsOptions) -> Self {
        CourseSetOptions::default().with_threshold(js.threshold * M)
    }
}

#[wasm_bindgen]
pub fn convert_gpx_to_fit_bytes(gpx_input: &[u8], options: JsValue) -> Result<JsValue> {
    let mut set_options = CourseSetOptions::default();
    let mut fit_options = FitCourseOptions::default();
    if let Some(options) = serde_wasm_bindgen::from_value(options).ok() {
        set_options = CourseSetOptions::from(&options);
        fit_options = FitCourseOptions::from(&options);
    }

    let mut fit_output = Vec::new();
    let info = convert_gpx_to_fit(
        Cursor::new(gpx_input),
        &mut fit_output,
        set_options,
        fit_options,
    )?;

    let report =
        coursepointer::internal::report::conversion_report::<Kilometer<f64>>(info.clone())?;

    let info = JsConversionInfo {
        course_name: info.course_name.unwrap_or_default(),
        total_distance_m: info.total_distance.value_unsafe,
        num_waypoints: info.num_waypoints,
        course_points: info.course_points.into_iter().map(Into::into).collect(),
        fit_bytes: fit_output.into_boxed_slice(),
        report,
    };
    Ok(serde_wasm_bindgen::to_value(&info)?)
}

#[derive(Serialize)]
#[wasm_bindgen(getter_with_clone)]
pub struct EnumVariant {
    pub name: String,
    pub value: u8,
}

#[wasm_bindgen]
pub fn enumerate_sports() -> Result<Vec<JsValue>> {
    let mut result = Vec::new();
    for variant in Sport::iter() {
        let repr = EnumVariant {
            name: variant.to_string(),
            value: variant as u8,
        };
        result.push(serde_wasm_bindgen::to_value(&repr)?);
    }
    Ok(result)
}

#[wasm_bindgen]
pub fn demo_course_set() -> Result<JsCourse> {
    (|| -> Result<JsCourse> {
        let mut builder = CourseSetBuilder::new(CourseSetOptions::default());
        builder
            .add_route()
            .with_name("Demo route".to_owned())
            .with_route_point(GeoPoint::new(1.1 * DEG, 2.2 * DEG, None)?)
            .with_route_point(GeoPoint::new(3.3 * DEG, 4.4 * DEG, None)?);
        Ok(builder.build()?.courses.get(0).unwrap().clone().into())
    })()
}

#[wasm_bindgen]
pub fn direct_lon(lat1: f64, lon1: f64, azi1: f64, s12: f64) -> f64 {
    let p1 = GeoPoint::new(lat1 * DEG, lon1 * DEG, None).unwrap();
    coursepointer::internal::geodesic_direct(&p1, azi1 * DEG, s12 * M)
        .unwrap()
        .point2
        .lon()
        .value_unsafe
}
