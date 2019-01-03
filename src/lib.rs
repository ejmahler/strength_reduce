
use std::ops::{Div, Rem};

macro_rules! strength_reduced_impl {
    ($struct_name:ident, $primitive_type:ident, $intermediate_type:ident, $bit_width:expr) => (
        #[derive(Clone, Copy, Debug)]
        pub struct $struct_name {
            multiplier: $primitive_type,
            divisor: $primitive_type,
            shift_value: u8,
        }
        impl $struct_name {
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

            #[inline]
            pub fn div_rem(numerator: $primitive_type, denom: Self) -> ($primitive_type, $primitive_type) {
                let quotient = numerator / denom;
                let remainder = numerator - quotient * denom.divisor;
                (quotient, remainder)
            }

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
        #[derive(Clone, Copy, Debug)]
        pub struct $struct_name {
            multiplier: $intermediate_type,
            divisor: $primitive_type,
            shift_value: u8,
        }
        impl $struct_name {
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

            #[inline]
            pub fn div_rem(numerator: $primitive_type, denom: Self) -> ($primitive_type, $primitive_type) {
                let quotient = numerator / denom;
                let remainder = numerator - quotient * denom.divisor;
                (quotient, remainder)
            }

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

#[cfg(test)]
mod unit_tests {
    use super::*;

    macro_rules! reduction_test {
        ($test_name:ident, $struct_name:ident, $primitive_type:ident) => (
            #[test]
            fn $test_name() {
                let divisors: Vec<$primitive_type> =   (1..20).chain([std::$primitive_type::MAX - 1, std::$primitive_type::MAX].iter().map(|item| *item)).collect();
                let numerators: Vec<$primitive_type> = (0..20).chain([std::$primitive_type::MAX - 1, std::$primitive_type::MAX].iter().map(|item| *item)).collect();

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
}
