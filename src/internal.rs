//! Exports intended for internal use only.
//!
//! These need to be exported for access from the main CLI and the
//! `integration-stub` binary, but they are not intended for use by external
//! code. This module's API may change without semantic versioning!

pub use crate::fit::PROFILE_VERSION;
pub use crate::measure::{Kilometer, Mile};
