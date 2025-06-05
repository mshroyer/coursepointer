//! Units of measure
//! 
//! Extends the SI units of measure we use fom [`dimensioned`] with
//! dimensionless angular types, as well as a special type for [`Centimeter`],
//! which is used heavily in FIT encoding.

use std::ops::{Add, AddAssign, Div};

use approx::{AbsDiffEq, RelativeEq, relative_eq};
use dimensioned::si::Meter;
use num_traits::{Float, Num};

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
}

// Angular units:
unit_of_measure![Degree];
unit_of_measure![Semicircle];

unit_of_measure![Centimeter];

impl<N> From<Meter<N>> for Centimeter<N>
where
    N: Num + From<i32>,
{
    fn from(value: Meter<N>) -> Self {
        Self(N::from(100) * value.value_unsafe)
    }
}
