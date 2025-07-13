use coursepointer::{GeoPoint, DEG};
use dimensioned::si::M;
use thiserror::Error;
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
}

pub type Result<T> = std::result::Result<T, WasmWrapperError>;

// #[wasm_bindgen]
// pub fn demo_course_set() -> f64 {
//     (|| -> Result<f64> {
//         let mut builder = CourseSetBuilder::new(CourseSetOptions::default());
//         builder
//             .add_route()
//             .with_name("Demo route".to_owned())
//             .with_route_point(GeoPoint::new(1.1 * DEG, 2.2 * DEG, None)?)
//             .with_route_point(GeoPoint::new(3.3 * DEG, 4.4 * DEG, None)?);
//         Ok(builder
//             .build()?
//             .courses
//             .get(0)
//             .unwrap()
//             .total_distance()
//             .value_unsafe)
//     })()
//     .unwrap()
// }

#[wasm_bindgen]
pub fn direct_lon(lat1: f64, lon1: f64, azi1: f64, s12: f64) -> f64 {
    let p1 = GeoPoint::new(lat1 * DEG, lon1 * DEG, None).unwrap();
    coursepointer::internal::geodesic_direct(&p1, azi1 * DEG, s12 * M)
        .unwrap()
        .point2
        .lon()
        .value_unsafe
}
