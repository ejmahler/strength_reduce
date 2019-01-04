#[macro_use]
extern crate proptest;
extern crate strength_reduce;
extern crate num_integer;

use proptest::test_runner::Config;
use strength_reduce::{StrengthReducedU8, StrengthReducedU16, StrengthReducedU32, StrengthReducedU64, StrengthReducedUsize};


macro_rules! reduction_proptest {
    ($test_name:ident, $struct_name:ident, $primitive_type:ident) => (
        mod $test_name {
            use super::*;
            use proptest::sample::select;

            fn assert_div_rem_equivalence(divisor: $primitive_type, numerator: $primitive_type) {
                let reduced_divisor = $struct_name::new(divisor);
                let expected_div = numerator / divisor;
                let expected_rem = numerator % divisor;
                let reduced_div = numerator / reduced_divisor;
                let reduced_rem = numerator % reduced_divisor;
                assert_eq!(expected_div, reduced_div, "Divide failed with numerator: {}, divisor: {}", numerator, divisor);
                assert_eq!(expected_rem, reduced_rem, "Modulo failed with numerator: {}, divisor: {}", numerator, divisor);
                let (reduced_combined_div, reduced_combined_rem) = $struct_name::div_rem(numerator, reduced_divisor);
                assert_eq!(expected_div, reduced_combined_div, "div_rem divide failed with numerator: {}, divisor: {}", numerator, divisor);
                assert_eq!(expected_rem, reduced_combined_rem, "div_rem modulo failed with numerator: {}, divisor: {}", numerator, divisor);
            }



            proptest! {
                #![proptest_config(Config::with_cases(100_000))]

                #[test]
                fn fully_generated_inputs_are_div_rem_equivalent(divisor in 1..core::$primitive_type::MAX, numerator in 0..core::$primitive_type::MAX) {
                    assert_div_rem_equivalence(divisor, numerator);
                }

                #[test]
                fn generated_divisors_with_edge_case_numerators_are_div_rem_equivalent(
                        divisor in 1..core::$primitive_type::MAX,
                        numerator in select(vec![0 as $primitive_type, 1 as $primitive_type, core::$primitive_type::MAX - 1, core::$primitive_type::MAX])) {
                    assert_div_rem_equivalence(divisor, numerator);
                }

                #[test]
                fn generated_numerators_with_edge_case_divisors_are_div_rem_equivalent(
                        divisor in select(vec![1 as $primitive_type, 2 as $primitive_type, core::$primitive_type::MAX - 1, core::$primitive_type::MAX]),
                        numerator in 0..core::$primitive_type::MAX) {
                    assert_div_rem_equivalence(divisor, numerator);
                }
            }
        }
    )
}
reduction_proptest!(strength_reduced_u32, StrengthReducedU32, u32);
reduction_proptest!(strength_reduced_u64, StrengthReducedU64, u64);
reduction_proptest!(strength_reduced_usize, StrengthReducedUsize, usize);

macro_rules! exhaustive_test {
    ($test_name:ident, $struct_name:ident, $primitive_type:ident) => (
    	#[test]
    	fn $test_name() {
    		for divisor in 1..=std::$primitive_type::MAX {
    			let reduced_divisor = $struct_name::new(divisor);

    			for numerator in 0..=std::$primitive_type::MAX {
    				let expected_div = numerator / divisor;
	                let expected_rem = numerator % divisor;

	                let reduced_div = numerator / reduced_divisor;
	                assert_eq!(expected_div, reduced_div, "Divide failed with numerator: {}, divisor: {}", numerator, divisor);

	                let reduced_rem = numerator % reduced_divisor;
	                assert_eq!(expected_rem, reduced_rem, "Modulo failed with numerator: {}, divisor: {}", numerator, divisor);

	                let (reduced_combined_div, reduced_combined_rem) = $struct_name::div_rem(numerator, reduced_divisor);
	                assert_eq!(expected_div, reduced_combined_div, "div_rem divide failed with numerator: {}, divisor: {}", numerator, divisor);
	                assert_eq!(expected_rem, reduced_combined_rem, "div_rem modulo failed with numerator: {}, divisor: {}", numerator, divisor);
    			}
    		}
    	}
    )
}

exhaustive_test!(test_strength_reduced_u8_exhaustive, StrengthReducedU8, u8);
exhaustive_test!(test_strength_reduced_u16_exhaustive, StrengthReducedU16, u16);

#[test]
fn test_u8_spot() {
	let mut expected_failures = 0;
	let mut surprise_failures = 0;
	let mut false_negatives = 0;
	let mut correct = 0;

	for divisor in 1..=255 {
		if divisor & 1023 == 0 {
			println!("divisor: {}", divisor);
		}
		let reduced_divisor = StrengthReducedU8::new(divisor);

		let predict_failure = if !divisor.is_power_of_two() {
			let divisor_u32 = divisor as u32;
			let bit_width = 8;
			let floor_log: u32 = bit_width - divisor.leading_zeros() - 1;

			let (_multiplier, rem) = num_integer::div_rem((1 << (floor_log + bit_width)), divisor_u32);
			let error = divisor as u32 - rem;

			error >= (1 << floor_log)
		} else {
			false
		};

		let mut failed = false;
		for numerator in 0..=255 {
			let expected_div = numerator / divisor;
			let actual_div = numerator / reduced_divisor;

			if expected_div != actual_div {
				failed = true;
				break;
			}
		}

		if failed {
			if predict_failure {
				expected_failures += 1;
			} else {
				surprise_failures += 1;
			}
		} else {
			if predict_failure {
				false_negatives += 1;
			}
			else {
				correct += 1;
			}
		}
	}

	println!("expected failures: {:?}", expected_failures);
	println!("surprise failures: {:?}", surprise_failures);
	println!("false negatives:   {:?}", false_negatives);
	println!("correct:           {:?}", correct);
}