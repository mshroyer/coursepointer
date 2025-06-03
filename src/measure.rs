//! Simple zero-overhead unit of measure types
//!
//! A poor man's version of F#'s units of measure, in order to keep units
//! correct by construction.  I wrote these rather than use the popular `uom`
//! crate because the latter obscures the actual storage unit and numeric type.

use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Div, Mul};

use approx::{AbsDiffEq, RelativeEq, relative_eq};
use num_traits::{Float, Num, NumCast};

trait IntoDimBase {
    type DimBase;

    fn into_dim_base(self) -> Self::DimBase;
}

trait FromUnit<U> {
    fn from_unit(u: U) -> Self;
}

trait IntoUnit<U> {
    fn into_unit(self) -> U;
}

trait CastUnitFrom<U>
where
    Self: Sized,
{
    fn cast_from(value: U) -> Option<Self>;
}

trait CastUnitInto<U> {
    fn cast_into(self) -> Option<U>;
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

        // Numeric casting

        impl<M, N> CastUnitFrom<$u<M>> for $u<N>
        where
            N: Num + NumCast,
            M: Num + NumCast,
            Self: Sized,
        {
            fn cast_from(value: $u<M>) -> Option<Self> {
                NumCast::from(value.0).map(Self)
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

    ($u:ident, $coeff:tt * $base:ident) => {
        unit_of_measure!($u);
        unit_conversion!($u, $coeff, $base);
    };

    ($u:ident, $base:ident * $coeff:tt) => {
        unit_of_measure!($u);
        unit_conversion!($u, $coeff, $base);
    };

    ($u:ident, $base:ident / $coeff:tt) => {
        unit_of_measure!($u);
        unit_conversion!($u, 1.0 / ($coeff as f64), $base);
    };
}

macro_rules! unit_conversion {
    ($u:ident, $coeff:expr, $base:ident) => {
        impl<N> FromUnit<$u<N>> for $base<f64>
        where
            N: Num,
            f64: From<N>,
        {
            fn from_unit(value: $u<N>) -> Self {
                Self(<f64 as From<N>>::from(value.0) * ($coeff as f64))
            }
        }

        impl<N> FromUnit<$base<N>> for $u<f64>
        where
            N: Num,
            f64: From<N>,
        {
            fn from_unit(value: $base<N>) -> Self {
                Self(<f64 as From<N>>::from(value.0) / ($coeff as f64))
            }
        }

        impl<N> IntoDimBase for $u<N>
        where
            N: Num,
            f64: From<N>,
        {
            type DimBase = $base<f64>;

            fn into_dim_base(self) -> Self::DimBase {
                Self::DimBase::from_unit(self)
            }
        }
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

impl<U, V> CastUnitInto<U> for V
where
    U: CastUnitFrom<V>,
{
    fn cast_into(self) -> Option<U> {
        U::cast_from(self)
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
unit_of_measure![Hours, Seconds * 3600];

// Distance units:
unit_of_measure![Meters];
unit_of_measure![Centimeters, Meters / 100];

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

    #[test]
    fn unit_addition() {
        let a = Meters(2);
        let b = Meters(3);
        assert_eq!(a + b, Meters(5));
    }

    #[test]
    fn convert_meters_to_cm() {
        assert_eq!(Centimeters::from(Meters(5)), Centimeters(500));
    }

    unit_of_measure!(Minutes, 60 * Seconds);

    #[test]
    fn unit_ratio() {
        assert_eq!(MetersPerSecond(3), Meters(6) / Seconds(2));
        assert_eq!(Seconds(2), Meters(6) / MetersPerSecond(3));
        assert_eq!(Meters(6), Seconds(2) * MetersPerSecond(3));
        assert_eq!(Meters(6), MetersPerSecond(3) * Seconds(2));
    }

    #[test]
    fn from_unit_conversions() {
        assert_eq!(Seconds::from_unit(Hours(1.0)), Seconds(3600.0));
        assert_eq!(Minutes::from_unit(Hours(2.0)), Minutes(120.0));
    }

    #[test]
    fn into_unit_conversions() {
        let seconds: Seconds<f64> = Hours(3.0).into_unit();
        assert_eq!(seconds, Seconds(10800.0));
    }

    #[test]
    fn convert_integer_values() {
        assert_eq!(Seconds(60.0), Seconds::from_unit(Minutes(1)));
    }

    #[test]
    fn convert_divisor_coefficient() {
        assert_eq!(Centimeters(250.0), Centimeters::from_unit(Meters(2.5)));
    }

    #[test]
    fn cast_into() {
        let x: Option<Meters<u32>> = Meters(42.0).cast_into();
        assert_eq!(x, Some(Meters(42)));

        let y: Option<Seconds<u8>> = Seconds(10000.0).cast_into();
        assert_eq!(y, None);

        let z: Option<Meters<u32>> = Meters(12.6).cast_into();
        assert_eq!(z, Some(Meters(12)));
    }
}
