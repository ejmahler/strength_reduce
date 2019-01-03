//! `strength_reduce` implements integer division and modulo via "arithmetic strength reduction"
//!
//! This results in much better performance when computing repeated divisions or modulos.
//!
//! # Example:
//! ```
//! use strength_reduce::StrengthReducedU64;
//! 
//! let mut my_array: Vec<u64> = (0..500).collect();
//! let divisor = 3;
//! let modulo = 14;
//!
//! // slow naive division and modulo
//! for element in &mut my_array {
//!     *element = (*element / divisor) % modulo;
//! }
//!
//! // fast strength-reduced division and modulo
//! let reduced_divisor = StrengthReducedU64::new(divisor);
//! let reduced_modulo = StrengthReducedU64::new(modulo);
//! for element in &mut my_array {
//!     *element = (*element / reduced_divisor) % reduced_modulo;
//! }
//! ```
//!
//! The intended use case for StrengthReducedU## is for use in hot loops like the one in the example above:
//! A division is repeated hundreds of times in a loop, but the divisor remains unchanged. In these cases,
//! strength-reduced division and modulo are 5x-10x faster than naive division and modulo.
//!
//! Benchmarking suggests that for u8, u16, and u32, on a x64 windows PC, using StrengthReducedU## is
//! **always** faster than naive division or modulo, even when not used inside a loop.
//! For u64, it's slower if it's only used a few times, due to nontrivial setup costs, with a break-even point around 10-20.
//!
//! The optimizations that this library provides are inherently dependent on architecture, compiler, and platform,
//! so test before you use. 
#![no_std]

use core::ops::{Div, Rem};

macro_rules! strength_reduced_impl {
    ($struct_name:ident, $primitive_type:ident, $intermediate_type:ident, $bit_width:expr) => (
        /// Implements unsigned division and modulo via mutiplication and shifts.
        ///
        /// Creating a an instance of this struct is more expensive than a single division, but if the division is repeated,
        /// this version will be several times faster than naive division.
        #[derive(Clone, Copy, Debug)]
        pub struct $struct_name {
            multiplier: $primitive_type,
            divisor: $primitive_type,
            shift_value: u8,
        }
        impl $struct_name {
            /// Creates a new divisor instance.
            ///
            /// If possible, avoid calling new() from an inner loop: The intended usage is to create an instance of this struct outside the loop, and use it for divison and remainders inside the loop.
            ///
            /// # Panics:
            /// 
            /// Panics if `divisor` is 0
            #[inline]
            pub fn new(divisor: $primitive_type) -> Self {
                assert!(divisor > 0);
                if divisor == 1 { 
                    Self{ multiplier: 1, divisor, shift_value: 0 }
                } else {
                    let big_divisor = divisor as $intermediate_type;
                    let trailing_zeros = big_divisor.next_power_of_two().trailing_zeros();
                    let shift_size = trailing_zeros + $bit_width - 1;

                    Self {
                        multiplier: (((1 << shift_size) + big_divisor - 1) / big_divisor) as $primitive_type,
                        divisor,
                        shift_value: shift_size as u8
                    }
                }
            }

            /// Simultaneous truncated integer division and modulus.
            /// Returns `(quotient, remainder)`.
            #[inline]
            pub fn div_rem(numerator: $primitive_type, denom: Self) -> ($primitive_type, $primitive_type) {
                let quotient = numerator / denom;
                let remainder = numerator - quotient * denom.divisor;
                (quotient, remainder)
            }

            /// Retrieve the value used to create this struct
            #[inline]
            pub fn get(&self) -> $primitive_type {
                self.divisor
            }
        }

        impl Div<$struct_name> for $primitive_type {
            type Output = $primitive_type;

            #[inline]
            fn div(self, rhs: $struct_name) -> Self::Output {
                let multiplied = (self as $intermediate_type) * (rhs.multiplier as $intermediate_type);
                let shifted = multiplied >> rhs.shift_value;
                shifted as $primitive_type
            }
        }

        impl Rem<$struct_name> for $primitive_type {
            type Output = $primitive_type;

            #[inline]
            fn rem(self, rhs: $struct_name) -> Self::Output {
                let quotient = self / rhs;
                self - quotient * rhs.divisor
            }
        }
    )
}


// in the "intermediate_multiplier" version, we store the mutiplier as the intermediate type instead of as the primitive type, and the mutiply routine is slightly more complicated
macro_rules! strength_reduced_impl_intermediate_multiplier {
    ($struct_name:ident, $primitive_type:ident, $intermediate_type:ident, $bit_width:expr) => (
        /// Implements unsigned division and modulo via mutiplication and shifts.
        ///
        /// Creating a an instance of this struct is more expensive than a single division, but if the division is repeated,
        /// this version will be several times faster than naive division.
        #[derive(Clone, Copy, Debug)]
        pub struct $struct_name {
            multiplier: $intermediate_type,
            divisor: $primitive_type,
            shift_value: u8,
        }
        impl $struct_name {
            /// Creates a new divisor instance.
            ///
            /// If possible, avoid calling new() from an inner loop: The intended usage is to create an instance of this struct outside the loop, and use it for divison and remainders inside the loop.
            ///
            /// # Panics:
            /// 
            /// Panics if `divisor` is 0
            #[inline]
            pub fn new(divisor: $primitive_type) -> Self {
                assert!(divisor > 0);
                if divisor == 1 { 
                    Self{ multiplier: 1 << $bit_width, divisor, shift_value: 0 }
                } else {
                    let big_divisor = divisor as $intermediate_type;
                    let trailing_zeros = big_divisor.next_power_of_two().trailing_zeros();

                    Self {
                        multiplier: ((1 << trailing_zeros + $bit_width - 1) + big_divisor - 1) / big_divisor,
                        divisor,
                        shift_value: (trailing_zeros - 1) as u8
                    }
                }
            }

            /// Simultaneous truncated integer division and modulus.
            /// Returns `(quotient, remainder)`.
            #[inline]
            pub fn div_rem(numerator: $primitive_type, denom: Self) -> ($primitive_type, $primitive_type) {
                let quotient = numerator / denom;
                let remainder = numerator - quotient * denom.divisor;
                (quotient, remainder)
            }

            /// Retrieve the value used to create this struct
            #[inline]
            pub fn get(&self) -> $primitive_type {
                self.divisor
            }
        }

        impl Div<$struct_name> for $primitive_type {
            type Output = $primitive_type;

            #[inline]
            fn div(self, rhs: $struct_name) -> Self::Output {
                let multiplied = ((self as $intermediate_type) * rhs.multiplier) >> $bit_width;
                (multiplied as $primitive_type) >> rhs.shift_value
            }
        }

        impl Rem<$struct_name> for $primitive_type {
            type Output = $primitive_type;

            #[inline]
            fn rem(self, rhs: $struct_name) -> Self::Output {
                let quotient = self / rhs;
                self - quotient * rhs.divisor
            }
        }
    )
}

// We have two separate macros because the two bigger versions seem to want to be optimized in a slightly different way than the two smaller ones
strength_reduced_impl!(StrengthReducedU8, u8, u16, 8);
strength_reduced_impl!(StrengthReducedU16, u16, u32, 16);
strength_reduced_impl_intermediate_multiplier!(StrengthReducedU32, u32, u64, 32);
strength_reduced_impl_intermediate_multiplier!(StrengthReducedU64, u64, u128, 64);

// Our definition for usize will depend on how big usize is
#[cfg(target_pointer_width = "16")]
strength_reduced_impl!(StrengthReducedUsize, usize, u32, 16);
#[cfg(target_pointer_width = "32")]
strength_reduced_impl_intermediate_multiplier!(StrengthReducedUsize, usize, u64, 32);
#[cfg(target_pointer_width = "64")]
strength_reduced_impl_intermediate_multiplier!(StrengthReducedUsize, usize, u128, 64);




#[cfg(test)]
mod unit_tests {
    use super::*;

    macro_rules! reduction_test {
        ($test_name:ident, $struct_name:ident, $primitive_type:ident) => (
            #[test]
            fn $test_name() {
                let max = core::$primitive_type::MAX;
                let divisors = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,max-1,max];
                let numerators = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,max-1,max];

                for &divisor in &divisors {
                    let reduced_divisor = $struct_name::new(divisor);
                    for &numerator in &numerators {
                        let expected_div = numerator / divisor;
                        let expected_rem = numerator % divisor;

                        let reduced_div = numerator / reduced_divisor;
                        let reduced_rem = numerator % reduced_divisor;

                        let (reduced_combined_div, reduced_combined_rem) = $struct_name::div_rem(numerator, reduced_divisor);

                        assert_eq!(expected_div, reduced_div, "Divide failed with numerator: {}, divisor: {}", numerator, divisor);
                        assert_eq!(expected_rem, reduced_rem, "Modulo failed with numerator: {}, divisor: {}", numerator, divisor);
                        assert_eq!(expected_div, reduced_combined_div, "div_rem divide failed with numerator: {}, divisor: {}", numerator, divisor);
                        assert_eq!(expected_rem, reduced_combined_rem, "div_rem modulo failed with numerator: {}, divisor: {}", numerator, divisor);
                    }
                }
            }
        )
    }

    reduction_test!(test_strength_reduced_u8, StrengthReducedU8, u8);
    reduction_test!(test_strength_reduced_u16, StrengthReducedU16, u16);
    reduction_test!(test_strength_reduced_u32, StrengthReducedU32, u32);
    reduction_test!(test_strength_reduced_u64, StrengthReducedU64, u64);
    reduction_test!(test_strength_reduced_usize, StrengthReducedUsize, usize);
}
