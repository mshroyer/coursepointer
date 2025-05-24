//! Simple zero-overhead unit of measure types
//!
//! A poor man's version of F#'s units of measure, in order to keep units
//! correct by construction.  I wrote these rather than use the popular `uom`
//! crate because the latter obscures the actual storage unit and numeric
//! type.

use num_traits::Num;

/// An angle in degrees.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Degrees<N: Num>(pub N);

/// A length in meters.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Meters<N: Num>(pub N);
