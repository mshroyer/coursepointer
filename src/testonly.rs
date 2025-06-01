//! Exports intended for testing use only.
//!
//! These need to be exported for access from the `integration-stub` binary
//! crate, but they are not intended for use by external code.

pub use crate::course::CourseSetBuilder;
pub use crate::fit::{CourseFile, PROFILE_VERSION};
pub use crate::measure::{Degrees, KilometersPerHour};
pub use crate::types::GeoPoint;
