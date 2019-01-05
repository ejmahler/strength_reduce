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
//! For divisors that are known at compile-time, the compiler is already capable of performing arithmetic strength reduction.
//! But if the divisor is only known at runtime, the compiler cannot optimize away the division. `strength_reduce` is designed
//! for situations where the divisor is not known until runtime.
//! 
//! `strength_reduce` is `#![no_std]`
//!
//! The optimizations that this library provides are inherently dependent on architecture, compiler, and platform,
//! so test before you use. 
#![no_std]

use core::ops::{Div, Rem};

#[derive(Clone, Copy, Debug)]
enum UnsignedDivisionAlgorithm {
    // Shift the numerator, but don't do anything else to it. Used for powers of two.
    ShiftOnly,

    // Multiply the numerator, then shift it
    MutiplyAndShift,

    // Same as MiltiplyAndShift, except there is an implicit added bit that's been truncated off of the multiplier
    // (Example: for u8, this says the multiplier is treated like 9 bits, where the MSB is 1 but has been truncated)
    // For some divisors, the primitive type sadly doesn't have enough bits to store the multiplier
    ExtraMultiplyBit,
}
use UnsignedDivisionAlgorithm::*;

// small types prefer to do work in the intermediate type
macro_rules! strength_reduced_impl_small {
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
            algorithm: UnsignedDivisionAlgorithm,
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

                // it will simplify the rest of this method if we have a div_rem that takes intermediate types, and returns primitive types
                let div_rem = |numerator: $intermediate_type, denominator: $intermediate_type| {
                    let quotient = numerator / denominator;
                    let remainder = numerator - quotient * denominator;
                    (quotient as $primitive_type, remainder as $primitive_type)
                };

                
                if divisor.is_power_of_two() { 
                    Self{ multiplier: 1, divisor, shift_value: divisor.trailing_zeros() as u8, algorithm: ShiftOnly }
                } else {
                    let shift_size = $bit_width - divisor.leading_zeros() - 1;

                    // to determine our multiplier, we're going to divide a big power of 2 by our divisor
                    let (multiplier, remainder) = div_rem(1 << (shift_size + $bit_width), divisor as $intermediate_type);
                    
                    // Before we commit to using this multiplier and shift value, check the remainder of the division we used to get our multiplier.
                    // For some divisors, this multiplier won't be big enough, and the remainder will tell us if that's happened
                    let error = divisor - remainder;
                    if error >= (1 << shift_size) {
                        // we've found a case where the multiplier isn't big enough (ie it doesn't have enough precision). if we proceed with it as shown,
                        // we will get numerators in the upper half of the space (ie, for u8, we'll get numerators > 127) where the quotient is off by one from the correct value
                        // We can double the multiplier for extra precision, but this will cause the multiplier to wrap.
                        // so we're going to use the ExtraMultiplyBit enum value to make it clear that our multiplier has wrapped
                        Self {
                            multiplier: multiplier.wrapping_shl(1) + 1,
                            divisor,
                            shift_value: shift_size as u8 + 1,
                            algorithm: ExtraMultiplyBit,
                        }
                    }
                    else {
                        // we're satisfied that the multiplier has enough precision
                        Self {
                            multiplier: multiplier + 1,
                            divisor,
                            shift_value: (shift_size + $bit_width) as u8,
                            algorithm: MutiplyAndShift,
                        }
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
                match rhs.algorithm {
                    ShiftOnly => self >> rhs.shift_value,
                    MutiplyAndShift => {
                        let multiplied = (self as $intermediate_type) * (rhs.multiplier as $intermediate_type);
                        (multiplied >> rhs.shift_value) as $primitive_type
                    },
                    ExtraMultiplyBit => {
                        let multiplied = (self as $intermediate_type) * (rhs.multiplier as $intermediate_type);
                        let upper_product = multiplied >> $bit_width;

                        // note that the multiplier is wrapped -- so for u8, if rhs.multiplier is 37, then we're actually multiplying by (256 + 37)
                        // IE, we're doing 256 * numerator + 37 * numerator
                        // But since we immediately shift right by the bit width, which in this example is 8, we shift out the multiply by 256
                        // So we're left with numerator + (37 * numerator) >> bit_width). aka numerator + upper_product
                        // We have to make sure we do this addition in the intermediate type, because it could overflow the smaller type
                        let shifted = (self as $intermediate_type + upper_product) >> rhs.shift_value;
                        shifted as $primitive_type
                    },
                }
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
            algorithm: UnsignedDivisionAlgorithm,
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

                // it will simplify the rest of this method if we have a div_rem that takes intermediate types, and returns primitive types
                let div_rem = |numerator: $intermediate_type, denominator: $intermediate_type| {
                    let quotient = numerator / denominator;
                    let remainder = numerator - quotient * denominator;
                    (quotient as $primitive_type, remainder as $primitive_type)
                };

                
                if divisor.is_power_of_two() { 
                    Self{ multiplier: 1, divisor, shift_value: divisor.trailing_zeros() as u8, algorithm: ShiftOnly }
                } else {
                    let shift_size = $bit_width - divisor.leading_zeros() - 1;

                    // to determine our multiplier, we're going to divide a big power of 2 by our divisor
                    let (multiplier, remainder) = div_rem(1 << (shift_size + $bit_width), divisor as $intermediate_type);
                    
                    // Before we commit to using this multiplier and shift value, check the remainder of the division we used to get our multiplier.
                    // For some divisors, this multiplier won't be big enough, and the remainder will tell us if that's happened
                    let error = divisor - remainder;
                    if error >= (1 << shift_size) {
                        // we've found a case where the multiplier isn't big enough (ie it doesn't have enough precision). if we proceed with it as shown,
                        // we will get numerators in the upper half of the space (ie, for u8, we'll get numerators > 127) where the quotient is off by one from the correct value
                        // We can double the multiplier for extra precision, but this will cause the multiplier to wrap.
                        // so we're going to use the ExtraMultiplyBit enum value to make it clear that our multiplier has wrapped
                        Self {
                            multiplier: multiplier.wrapping_shl(1) + 1,
                            divisor,
                            shift_value: shift_size as u8,
                            algorithm: ExtraMultiplyBit,
                        }
                    }
                    else {
                        // we're satisfied that the multiplier has enough precision
                        Self {
                            multiplier: multiplier + 1,
                            divisor,
                            shift_value: shift_size as u8,
                            algorithm: MutiplyAndShift,
                        }
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
                match rhs.algorithm {
                    ShiftOnly => self >> rhs.shift_value,
                    MutiplyAndShift => {
                        let multiplied = (self as $intermediate_type) * (rhs.multiplier as $intermediate_type);
                        let upper_product = (multiplied >> $bit_width) as $primitive_type;
                        upper_product >> rhs.shift_value
                    },
                    ExtraMultiplyBit => {
                        let multiplied = (self as $intermediate_type) * (rhs.multiplier as $intermediate_type);
                        let upper_product = (multiplied >> $bit_width) as $primitive_type;

                        // note that the multiplier is wrapped -- so for u8, if rhs.multiplier is 37, then we're actually multiplying by (256 + 37)
                        // IE in this example we're doing 256 * numerator + 37 * numerator
                        // But since we immediately shift right by the bit width, we get, in the u8 example, (256 * numerator + 37 * numerator) / 256
                        // So we're left with numerator + (37 * numerator) >> bit_width). aka numerator + upper_product
                        // Unfortunately, if we just add numerator and upper_product, we might overflow. One solution is to divide by 2 before shifting, and then shift one less.
                        // It turns out that upper_product + (numerator - upper_product) / 2 is equivalent to (upper_product + numerator) / 2, but doesn't overflow!
                        // So we divide by 2 here, and to compensate, we shift one less than normal (shifting one less is handled in the constructor)
                        let half_difference = (self - upper_product) / 2;
                        (upper_product + half_difference) >> rhs.shift_value
                    }
                }
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

// u8 appears to be much faster strength_reduced_impl_small -- u16 sppears to be marginally faster with strength_reduced_impl, and the others are significantly faster with strength_reduced_impl
strength_reduced_impl_small!(StrengthReducedU8, u8, u16, 8);
strength_reduced_impl!(StrengthReducedU16, u16, u32, 16);
strength_reduced_impl!(StrengthReducedU32, u32, u64, 32);
strength_reduced_impl!(StrengthReducedU64, u64, u128, 64);

// Our definition for usize will depend on how big usize is
#[cfg(target_pointer_width = "16")]
strength_reduced_impl!(StrengthReducedUsize, usize, u32, 16);
#[cfg(target_pointer_width = "32")]
strength_reduced_impl!(StrengthReducedUsize, usize, u64, 32);
#[cfg(target_pointer_width = "64")]
strength_reduced_impl!(StrengthReducedUsize, usize, u128, 64);




#[cfg(test)]
mod unit_tests {
    use super::*;

    macro_rules! reduction_test {
        ($test_name:ident, $struct_name:ident, $primitive_type:ident) => (
            #[test]
            fn $test_name() {
                let max = core::$primitive_type::MAX;
                let divisors = [7,8,9,10,11,12,13,14,15,16,17,18,19,20,max-1,max];
                let numerators = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,max-1,max];

                for &divisor in &divisors {
                    let reduced_divisor = $struct_name::new(divisor);
                    for &numerator in &numerators {
                        let expected_div = numerator / divisor;
                        let expected_rem = numerator % divisor;

                        let reduced_div = numerator / reduced_divisor;
                        assert_eq!(expected_div, reduced_div, "Divide failed with numerator: {}, divisor: {}", numerator, divisor);
                        let reduced_rem = numerator % reduced_divisor;

                        let (reduced_combined_div, reduced_combined_rem) = $struct_name::div_rem(numerator, reduced_divisor);

                        
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
