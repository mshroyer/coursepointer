//! Simple zero-overhead unit of measure types
//!
//! A poor man's version of F#'s units of measure, in order to keep units
//! correct by construction.  I wrote these rather than use the popular `uom`
//! crate because the latter obscures the actual storage unit and numeric
//! type.

use std::ops::{AddAssign, Div};

use num_traits::Num;

trait UnitOfMeasure<N>
where
    N: Num + Copy,
{
    fn value(&self) -> N;
}

macro_rules! unit_of_measure {
    ($u:tt) => {
        #[derive(Clone, Copy, PartialEq, Debug)]
        pub struct $u<N: Num + Copy>(pub N);

        impl<N> UnitOfMeasure<N> for $u<N>
        where
            N: Num + Copy,
        {
            fn value(&self) -> N {
                self.0
            }
        }

        impl<N> AddAssign for $u<N>
        where
            N: Num + Copy + AddAssign,
        {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }
    }
}

// Time units:
unit_of_measure![Seconds];
unit_of_measure![Hours];

// Distance units:
unit_of_measure![Meters];
unit_of_measure![Centimeters];

// Angular units:
unit_of_measure![Degrees];
unit_of_measure![Semicircles];

// Velocity / speed units:
unit_of_measure![MetersPerSecond];
unit_of_measure![KilometersPerHour];

impl<N> From<Meters<N>> for Centimeters<N>
where
    N: Num + Copy + From<u8>,
{
    fn from(value: Meters<N>) -> Centimeters<N> {
        Centimeters(N::from(100u8) * value.0)
    }
}

impl<N> From<KilometersPerHour<N>> for MetersPerSecond<N>
where
    N: Num + Copy + From<u8>,
{
    fn from(value: KilometersPerHour<N>) -> Self {
        MetersPerSecond(value.0 * N::from(5u8) / N::from(18u8))
    }
}

impl<N> Div<Seconds<N>> for Meters<N>
where
    N: Num + Copy,
{
    type Output = MetersPerSecond<N>;

    fn div(self, rhs: Seconds<N>) -> Self::Output {
        MetersPerSecond(self.0 / rhs.0)
    }
}

impl<N> Div<MetersPerSecond<N>> for Meters<N>
where
    N: Num + Copy,
{
    type Output = Seconds<N>;

    fn div(self, rhs: MetersPerSecond<N>) -> Self::Output {
        Seconds(self.0 / rhs.0)
    }
}
