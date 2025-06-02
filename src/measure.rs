//! Simple zero-overhead unit of measure types
//!
//! A poor man's version of F#'s units of measure, in order to keep units
//! correct by construction.  I wrote these rather than use the popular `uom`
//! crate because the latter obscures the actual storage unit and numeric type.

use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Div, Mul};

use approx::{AbsDiffEq, RelativeEq, relative_eq};
use num_traits::{Float, Num, NumCast};

trait IntoDimBase
{
    type DimBase;

    fn into_dim_base(self) -> Self::DimBase;
}

trait FromDimBase
{
    type DimBase;

    fn from_dim_base(value: Self::DimBase) -> Self;
}

trait FromUnit<U>
{
    fn from_unit(u: U) -> Self;
}

trait IntoUnit<U>
{
    fn into_unit(self) -> U;
}

macro_rules! unit_of_measure {
    ($u:ident) => {
        #[derive(Clone, Copy, Default, PartialEq, PartialOrd, Debug)]
        pub struct $u<N: Num>(pub N);

        impl<N> Add for $u<N>
        where
            N: Num + Add,
        {
            type Output = Self;

            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl<N> AddAssign for $u<N>
        where
            N: Num + AddAssign,
        {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl<N> Div<N> for $u<N>
        where
            N: Num + Div,
        {
            type Output = Self;

            fn div(self, rhs: N) -> Self {
                Self(self.0 / rhs)
            }
        }

        // Relative equality traits for appox support

        impl<N> AbsDiffEq for $u<N>
        where
            N: Num + Float + AbsDiffEq<N, Epsilon = N>,
        {
            type Epsilon = N;

            fn default_epsilon() -> Self::Epsilon {
                N::epsilon()
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
                self.0.abs_diff_eq(&other.0, epsilon)
            }
        }

        impl<N> RelativeEq for $u<N>
        where
            N: Num + Float + AbsDiffEq<N, Epsilon = N> + RelativeEq<N>,
        {
            fn default_max_relative() -> Self::Epsilon {
                N::epsilon()
            }

            fn relative_eq(
                &self,
                other: &Self,
                epsilon: Self::Epsilon,
                max_relative: Self::Epsilon,
            ) -> bool {
                relative_eq!(
                    self.0,
                    other.0,
                    epsilon = epsilon,
                    max_relative = max_relative
                )
            }
        }
    };

    ($u:ident, ($coeff:expr, $base:ident)) => {
        unit_of_measure!($u);

        impl<N> FromUnit<$u<N>> for $base<N>
        where
            N: Num + NumCast,
        {
            fn from_unit(value: $u<N>) -> Self {
                // TODO: Find a way to remove this runtime panic?
                Self(value.0 * NumCast::from($coeff).expect("literal out of range for N"))
            }
        }

        impl<N> FromUnit<$base<N>> for $u<N>
        where
            N: Num + NumCast,
        {
            fn from_unit(value: $base<N>) -> Self {
                Self(value.0 / NumCast::from($coeff).expect("literal out of range for N"))
            }
        }

        impl<N> IntoDimBase for $u<N>
        where
            N: Num + NumCast,
        {
            type DimBase = $base<N>;

            fn into_dim_base(self) -> Self::DimBase {
                Self::DimBase::from_unit(self)
            }
        }

        // impl<N, V> From<V> for $u<N>
        // where
        //     N: Num + NumCast,
        //     V: IntoUnit<$u<N>>,
        // {
        //     fn from(v: V) -> Self {
        //         v.into_unit()
        //     }
        // }
        //
        // impl<N, V> From<V> for $base<N>
        // where
        //     N: Num + NumCast,
        //     V: IntoUnit<$base<N>>,
        // {
        //     fn from(value: V) -> Self {
        //         value.into_unit()
        //     }
        // }
        //
        // impl<N> From<$u<N>> for $base<N>
        // where
        //     N: Num + NumCast,
        // {
        //     fn from(value: $u<N>) -> Self {
        //         // TODO: Find a way to remove this runtime panic?
        //         Self(value.0 * NumCast::from($coeff).expect("literal out of range for N"))
        //     }
        // }
        //
        // impl<N> From<$base<N>> for $u<N>
        // where
        //     N: Num + NumCast,
        // {
        //     fn from(value: $base<N>) -> Self {
        //         Self(value.0 / NumCast::from($coeff).expect("literal out of range for N"))
        //     }
        // }
    };
}

impl<U, V> FromUnit<V> for U
where
    U: FromUnit<V::DimBase>,
    V: IntoDimBase,
{
    fn from_unit(v: V) -> Self {
        U::from_unit(v.into_dim_base())
    }
}

impl<U, V> IntoUnit<U> for V
where
    U: FromUnit<V>,
{
    fn into_unit(self) -> U {
        U::from_unit(self)
    }
}

macro_rules! unit_ratio_impl {
    ($ratio:ident, $num:ident, $denom:ident) => {
        impl<N> Div<$denom<N>> for $num<N>
        where
            N: Num + Div,
        {
            type Output = $ratio<N>;

            fn div(self, rhs: $denom<N>) -> Self::Output {
                $ratio(self.0 / rhs.0)
            }
        }

        impl<N> Mul<$denom<N>> for $ratio<N>
        where
            N: Num + Mul,
        {
            type Output = $num<N>;

            fn mul(self, rhs: $denom<N>) -> Self::Output {
                $num(self.0 * rhs.0)
            }
        }
    };
}

macro_rules! unit_ratio {
    ($ratio:ident, $num:ident, $denom:ident) => {
        unit_ratio_impl!($ratio, $num, $denom);
        unit_ratio_impl!($denom, $num, $ratio);
    };
}

// Time units:
unit_of_measure![Seconds];
unit_of_measure![Hours, (3600, Seconds)];

// Distance units:
unit_of_measure![Meters];
unit_of_measure![Centimeters];

// conversion_group![Meters, (100.0, Centimeters)];

// Angular units:
unit_of_measure![Degrees];
unit_of_measure![Semicircles];

// Velocity / speed units:
unit_of_measure![MetersPerSecond];
unit_of_measure![KilometersPerHour];

impl<N> From<Meters<N>> for Centimeters<N>
where
    N: Num + From<u8>,
{
    fn from(value: Meters<N>) -> Centimeters<N> {
        Centimeters(N::from(100u8) * value.0)
    }
}

impl<T> Display for Meters<T>
where
    T: Num + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}m", self.0)
    }
}

impl<N> From<KilometersPerHour<N>> for MetersPerSecond<N>
where
    N: Num + From<u8>,
{
    fn from(value: KilometersPerHour<N>) -> Self {
        MetersPerSecond(value.0 * N::from(5u8) / N::from(18u8))
    }
}

unit_ratio![MetersPerSecond, Meters, Seconds];

#[cfg(test)]
mod tests {
    use super::*;

    unit_of_measure!(Minutes, (60, Seconds));

    #[test]
    fn convert_meters_to_cm() {
        assert_eq!(Centimeters::from(Meters(5)), Centimeters(500));
    }

    #[test]
    fn unit_ratio() {
        assert_eq!(MetersPerSecond(3), Meters(6) / Seconds(2));
        assert_eq!(Seconds(2), Meters(6) / MetersPerSecond(3));
        assert_eq!(Meters(6), Seconds(2) * MetersPerSecond(3));
        assert_eq!(Meters(6), MetersPerSecond(3) * Seconds(2));
    }

    #[test]
    fn from_unit_conversions() {
        assert_eq!(Seconds::from_unit(Hours(1)), Seconds(3600));
        assert_eq!(Minutes::from_unit(Hours(2)), Minutes(120));
    }

    #[test]
    fn into_unit_conversions() {
        let seconds : Seconds<f64> = Hours(3.0).into_unit();
        assert_eq!(seconds, Seconds(10800.0));
    }

    // #[test]
    // fn from_conversions() {
    //     assert_eq!(Seconds::from(Hours(1)), Seconds(3600));
    //     assert_eq!(Minutes::from(Hours(2)), Minutes(120));
    // }
}
