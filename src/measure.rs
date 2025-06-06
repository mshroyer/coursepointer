//! Units of measure
//!
//! Extends the SI units of measure we use fom [`dimensioned`] with
//! dimensionless angular types, as well as a special type for [`Centimeter`],
//! which is used heavily in FIT encoding.

use std::ops::{Add, AddAssign, Div, Mul};

use approx::{AbsDiffEq, RelativeEq, relative_eq};
use dimensioned::si::{Meter, Second};
use num_traits::{Float, Num, NumCast, Pow, ToPrimitive};

macro_rules! unit_of_measure {
    ($a:ident as $u:ident) => {
        #[derive(Clone, Copy, Default, PartialEq, PartialOrd, Debug)]
        pub struct $u<N: Num> {
            pub value_unsafe: N,
        }

        impl<N> $u<N>
        where
            N: Num,
        {
            pub fn new(x: N) -> Self {
                $u { value_unsafe: x }
            }
        }

        impl<N> $u<N>
        where
            N: Num + NumCast,
        {
            pub fn num_cast_from<M: Num + ToPrimitive>(val: $u<M>) -> Option<$u<N>> {
                <N as NumCast>::from(val.value_unsafe).map($u::new)
            }
        }

        #[allow(dead_code)]
        pub const $a: $u<u8> = $u { value_unsafe: 1u8 };

        impl<N> Add for $u<N>
        where
            N: Num + Add,
        {
            type Output = Self;

            fn add(self, rhs: Self) -> Self {
                Self::new(self.value_unsafe + rhs.value_unsafe)
            }
        }

        impl<N> AddAssign for $u<N>
        where
            N: Num + AddAssign,
        {
            fn add_assign(&mut self, rhs: Self) {
                self.value_unsafe += rhs.value_unsafe;
            }
        }

        // Work around the orphan rule to allow multiplication of a constant by
        // the unit abbreviation on the right-hand side, as in `1.0 * CM`.
        __constant_mul_impl!($u, f64);
        __constant_mul_impl!($u, f32);
        __constant_mul_impl!($u, i8);
        __constant_mul_impl!($u, i16);
        __constant_mul_impl!($u, i32);
        __constant_mul_impl!($u, i64);
        __constant_mul_impl!($u, u8);
        __constant_mul_impl!($u, u16);
        __constant_mul_impl!($u, u32);
        __constant_mul_impl!($u, u64);

        impl<N> Div<N> for $u<N>
        where
            N: Num + Div,
        {
            type Output = Self;

            fn div(self, rhs: N) -> Self {
                Self::new(self.value_unsafe / rhs)
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
                self.value_unsafe.abs_diff_eq(&other.value_unsafe, epsilon)
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
                    self.value_unsafe,
                    other.value_unsafe,
                    epsilon = epsilon,
                    max_relative = max_relative
                )
            }
        }
    };
}

macro_rules! __constant_mul_impl {
    ($u:ident, $n:tt) => {
        impl<N> Mul<$u<N>> for $n
        where
            N: Num,
            $n: From<N>,
        {
            type Output = $u<$n>;

            fn mul(self, rhs: $u<N>) -> Self::Output {
                Self::Output::new(self * <$n as ::std::convert::From<N>>::from(rhs.value_unsafe))
            }
        }
    };
}

// Angular units:
unit_of_measure![DEG as Degree];
unit_of_measure![SEMI as Semicircle];

unit_of_measure![CM as Centimeter];

unit_of_measure![MS as Millisecond];
unit_of_measure![NS as Nanosecond];

impl From<Degree<f64>> for Semicircle<f64> {
    fn from(value: Degree<f64>) -> Self {
        Self::new((2f64.pow(31) / 180.0) * value.value_unsafe)
    }
}

impl From<Semicircle<f64>> for Degree<f64> {
    fn from(value: Semicircle<f64>) -> Self {
        Self::new(value.value_unsafe * 180.0 / 2f64.powi(31))
    }
}

impl<N> From<Meter<N>> for Centimeter<N>
where
    N: Num + From<u8>,
{
    fn from(value: Meter<N>) -> Self {
        Self::new(N::from(100) * value.value_unsafe)
    }
}

impl<N> From<Second<N>> for Millisecond<N>
where
    N: Num + From<u16>,
{
    fn from(value: Second<N>) -> Self {
        Self::new(N::from(1_000) * value.value_unsafe)
    }
}

impl<N> From<Second<N>> for Nanosecond<N>
where
    N: Num + From<u32>,
{
    fn from(value: Second<N>) -> Self {
        Self::new(N::from(1_000_000_000) * value.value_unsafe)
    }
}
