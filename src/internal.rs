//! Exports intended for internal use only.
//!
//! These need to be exported for access from the main CLI and the
//! `integration-stub` binary crate, but they are not intended for use by
//! external code. This module's API may change without semantic versioning!

pub use crate::course::CourseSetBuilder;
pub use crate::fit::{CourseFile, PROFILE_VERSION};
pub use crate::measure::DEG;
pub use crate::types::GeoPoint;
