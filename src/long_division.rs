const U32_MAX: u64 = core::u32::MAX as u64;
const U64_MAX: u128 = core::u64::MAX as u128;

// divides a 128-bit number by a 128-bit divisor
pub fn divide_128(numerator: u128, divisor: u128) -> u128 {
	if divisor <= U64_MAX {
		let divisor64 = divisor as u64;
		let upper_numerator = (numerator >> 64) as u64;
		if divisor64 > upper_numerator {
			divide_128_by_64_helper(numerator, divisor64) as u128
		}
		else {
			let upper_quotient = upper_numerator / divisor64;
			let upper_remainder = upper_numerator - upper_quotient * divisor64;

			let intermediate_numerator = ((upper_remainder as u128) << 64) | (numerator as u64 as u128);
			let lower_quotient = divide_128_by_64_helper(intermediate_numerator, divisor64);

			((upper_quotient as u128) << 64) | (lower_quotient as u128)
		}
	}
	else {
		let shift_size = divisor.leading_zeros();
		let shifted_divisor = divisor << shift_size;

		let shifted_numerator = numerator >> 1;

		let upper_quotient = divide_128_by_64_helper(shifted_numerator, (shifted_divisor >> 64) as u64);
		let mut quotient = upper_quotient >> (63 - shift_size);
		if quotient > 0 {
			quotient -= 1;
		}

		let remainder = numerator - quotient as u128 * divisor;
		if remainder >= divisor {
			quotient += 1;
		}
		quotient as u128
	}
}

// divides a 128-bit number by a 64-bit divisor, returning the quotient as a 64-bit number. Panics if the quotient doesn't fit in a 64-bit number
fn divide_128_by_64_helper(numerator: u128, divisor: u64) -> u64 {
	// Assert that the upper half of the numerator is less than the denominator. This will guarantee that the quotient fits inside the numerator.
	// Sadly this will give us some false negatives! TODO: Find a quick test we can do that doesn't have false negatives
	// false negative example: numerator = u64::MAX * u64::MAX / u64::MAX
	assert!(divisor > (numerator >> 64) as u64, "The numerator is too large for the denominator; the quotient might not fit inside a u64.");

	if divisor <= U32_MAX {
		return divide_128_by_32_helper(numerator, divisor as u32);
	}

    let shift_size = divisor.leading_zeros();
	let shifted_divisor = divisor << shift_size;
	let shifted_numerator = numerator << shift_size;
	let divisor_hi = shifted_divisor >> 32;
    let divisor_lo = shifted_divisor as u32 as u64;

    // split the numerator into 3 chunks: the top 64-bits, the next 32-bits, and the lowest 32-bits
    let numerator_hi : u64 = (shifted_numerator >> 64) as u64;
    let numerator_mid : u64 = (shifted_numerator >> 32) as u32 as u64;
    let numerator_lo : u64 = shifted_numerator as u32 as u64;

    // we're essentially going to do a long division algorithm with 2 divisions, one on numerator_hi << 32 | numerator_mid, and the second on the remainder of the first | numerator_lo
    // but numerator_hi << 32 | numerator_mid is a 96-bit number, and we only have 64 bits to work with. so instead we split the divisor into 2 chunks, and divde by the upper chunk, and then check against the lower chunk in a while loop

    // step 1: divide the top chunk of the numerator by the divisor
    // IDEALLY, we would divide (numerator_hi << 32) | numerator_mid by shifted_divisor, but that would require a 128-bit numerator, which is the whole thing we're trying to avoid
    // so instead we're going to split the second division into two sub-phases. in 1a, we divide numerator_hi by divisor_hi, and then in 1b we decrement the quotient to account for the fact that it'll be smaller when you take divisor_lo into account

    // keep in mind that for all of step 2, the full numerator we're using will be
    // complete_first_numerator  = (numerator_midbits << 32) | numerator_mid

    // step 1a: divide the upper part of the middle numerator by the upper part of the divisor
    let mut quotient_hi = core::cmp::min(numerator_hi / divisor_hi, U32_MAX);
    let mut partial_remainder_hi = numerator_hi - quotient_hi * divisor_hi;

    // step 1b: we know sort of what the quotient is, but it's slightly too large because it doesn't account for divisor_lo, nor numerator_mid, so decrement the quotient until it fits
    // note that if we do some algebra on the condition in this while loop,
    // ie "quotient_hi * divisor_lo > (partial_remainder_hi << 32) | numerator_mid"
    // we end up getting "quotient_hi * shifted_divisor < (numerator_midbits << 32) | numerator_mid". remember that the right side of the inequality sign is complete_first_numerator from above.
    // which deminstrates that we're decrementing the quotient until the quotient multipled by the complete divisor is less than the complete numerator
    while partial_remainder_hi <= U32_MAX && quotient_hi * divisor_lo > (partial_remainder_hi << 32) | numerator_mid {
        quotient_hi -= 1;
        partial_remainder_hi += divisor_hi;
    }

    // step 2: Divide the bottom part of the numerator. We're going to have the same problem as step 1, where we want the numerator to be a 96-bit number, so again we're going to split it into 2 substeps
	// the full numeratoe for step 3 will be:
	// complete_second_numerator = (first_division_remainder << 32) | numerator_lo

    // step 2a: divide the upper part of the lower numerator by the upper part of the divisor
    // To get the numerator, complate the calculation of the full remainder by subtracing the quotient times the lower bits of the divisor
    // TODO: a warpping subtract is necessary here. why does this work, and why is it necessary?
    let full_remainder_hi = ((partial_remainder_hi << 32) | numerator_mid).wrapping_sub(quotient_hi * divisor_lo);

    let mut quotient_lo = core::cmp::min(full_remainder_hi / divisor_hi, U32_MAX);
    let mut partial_remainder_lo = full_remainder_hi - quotient_lo * divisor_hi;

    // step 2b: just like step 1b, decrement the final quotient until it's correctr when accounting for the full divisor
    while partial_remainder_lo <= U32_MAX && quotient_lo * divisor_lo > (partial_remainder_lo << 32) | numerator_lo {
        quotient_lo -= 1;
        partial_remainder_lo += divisor_hi;
    }

    // We now have our separate quotients, now we just have to add them together
    (quotient_hi << 32) | quotient_lo
}


// Same as divide_128_by_64_into_64, but optimized for scenarios where the divisor fits in a u32. Still panics if the quotient doesn't fit in a u64
fn divide_128_by_32_helper(numerator: u128, divisor: u32) -> u64 {
	// Assert that the upper half of the numerator is less than the denominator. This will guarantee that the quotient fits inside the numerator.
	// Sadly this will give us some false negatives! TODO: Find a quick test we can do that doesn't have false negatives
	// false negative example: numerator = u64::MAX * u64::MAX / u64::MAX
	assert!(divisor as u64 > (numerator >> 64) as u64, "The numerator is too large for the denominator; the quotient might not fit inside a u64.");

    let shift_size = divisor.leading_zeros();
	let shifted_divisor = (divisor << shift_size) as u64;
	let shifted_numerator = numerator << (shift_size + 32);

    // split the numerator into 3 chunks: the top 64-bits, the next 32-bits, and the lowest 32-bits
    let numerator_hi : u64 = (shifted_numerator >> 64) as u64;
    let numerator_mid : u64 = (shifted_numerator >> 32) as u32 as u64;

    // we're essentially going to do a long division algorithm with 2 divisions, one on numerator_hi << 32 | numerator_mid, and the second on the remainder of the first | numerator_lo
    // but numerator_hi << 32 | numerator_mid is a 96-bit number, and we only have 64 bits to work with. so instead we split the divisor into 2 chunks, and divde by the upper chunk, and then check against the lower chunk in a while loop

    // step 1: divide the top chunk of the numerator by the divisor
    // IDEALLY, we would divide (numerator_hi << 32) | numerator_mid by shifted_divisor, but that would require a 128-bit numerator, which is the whole thing we're trying to avoid
    // so instead we're going to split the second division into two sub-phases. in 1a, we divide numerator_hi by divisor_hi, and then in 1b we decrement the quotient to account for the fact that it'll be smaller when you take divisor_lo into account

    // keep in mind that for all of step 1, the full numerator we're using will be
    // complete_first_numerator  = (numerator_hi << 32) | numerator_mid

    // step 1a: divide the upper part of the middle numerator by the upper part of the divisor
    let quotient_hi = numerator_hi / shifted_divisor;
    let remainder_hi = numerator_hi - quotient_hi * shifted_divisor;

    // step 2: Divide the bottom part of the numerator. We're going to have the same problem as step 1, where we want the numerator to be a 96-bit number, so again we're going to split it into 2 substeps
	// the full numeratoe for step 3 will be:
	// complete_second_numerator = (first_division_remainder << 32) | numerator_lo

    // step 2a: divide the upper part of the lower numerator by the upper part of the divisor
    // To get the numerator, complate the calculation of the full remainder by subtracing the quotient times the lower bits of the divisor
    // TODO: a warpping subtract is necessary here. why does this work, and why is it necessary?
    let final_numerator = (remainder_hi) << 32 | numerator_mid;
    let quotient_lo = final_numerator / shifted_divisor;

    // We now have our separate quotients, now we just have to add them together
    (quotient_hi << 32) | quotient_lo
}



// divides a 256-bit number by a 128-bit divisor. returns (quotient_hi, quotient_lo)
pub fn divide_256_by_128(numerator_hi: u128, numerator_lo: u128, divisor: u128) -> (u128, u128) {
	if divisor > numerator_hi {
		let quotient_lo = divide_256_by_128_helper(numerator_hi, numerator_lo, divisor);
		(0, quotient_lo)
	}
	else {
		let quotient_hi = divide_128(numerator_hi, divisor);
		let remainder_hi = numerator_hi - quotient_hi * divisor;

		let quotient_lo = divide_256_by_128_helper(remainder_hi, numerator_lo, divisor);

		(quotient_hi, quotient_lo)
	}
}

// divides a 128-bit number by a 64-bit divisor, returning the quotient as a 64-bit number. Panics if the quotient doesn't fit in a 64-bit number
fn divide_256_by_128_helper(numerator_hi: u128, numerator_lo: u128, divisor: u128) -> u128 {
	// Assert that the upper half of the numerator is less than the denominator. This will guarantee that the quotient fits inside the numerator.
	// Sadly this will give us some false negatives! TODO: Find a quick test we can do that doesn't have false negatives
	// false negative example: numerator = u64::MAX * u64::MAX / u64::MAX
	assert!(divisor > numerator_hi, "The numerator is too large for the denominator; the quotient might not fit inside a u128.");

    let shift_size = divisor.leading_zeros();
	let shifted_divisor = divisor << shift_size;

	let shifted_numerator_hi = if shift_size > 0 { numerator_hi << shift_size | numerator_lo >> (128 - shift_size) } else { numerator_hi };
	let shifted_numerator_lo = numerator_lo << shift_size;


	let divisor_hi = shifted_divisor >> 64;
    let divisor_lo = shifted_divisor as u64 as u128;

    // split the numerator into 3 chunks: the top 64-bits, the next 32-bits, and the lowest 32-bits
    let inner_numerator_hi : u128 = shifted_numerator_hi;
    let inner_numerator_mid : u128 = (shifted_numerator_lo >> 64) as u128;
    let inner_numerator_lo : u128 = shifted_numerator_lo as u64 as u128;

    // we're essentially going to do a long division algorithm with 2 divisions, one on inner_numerator_hi << 64 | inner_numerator_mid, and the second on the remainder of the first | inner_numerator_lo
    // but inner_numerator_hi << 64 | inner_numerator_mid is a 192-bit number, and we only have 128 bits to work with. so instead we split the divisor into 2 chunks, and divde by the upper chunk, and then check against the lower chunk in a while loop

    // step 1: divide the top chunk of the numerator by the divisor
    // IDEALLY, we would divide (inner_numerator_hi << 64) | inner_numerator_mid by shifted_divisor, but that would require a 256-bit numerator, which is the whole thing we're trying to avoid
    // so instead we're going to split the second division into two sub-phases. in 1a, we divide inner_numerator_hi by divisor_hi, and then in 1b we decrement the quotient to account for the fact that it'll be smaller when you take divisor_lo into account

    // keep in mind that for all of step 1, the full numerator we're using will be
    // complete_first_numerator  = (inner_numerator_hi << 64) | inner_numerator_mid

    // step 1a: divide the upper part of the middle numerator by the upper part of the divisor
    let mut quotient_hi = core::cmp::min(divide_128(inner_numerator_hi, divisor_hi), U64_MAX);
    let mut partial_remainder_hi = inner_numerator_hi - quotient_hi * divisor_hi;

    // step 1b: we know sort of what the quotient is, but it's slightly too large because it doesn't account for divisor_lo, nor numerator_mid, so decrement the quotient until it fits
    // note that if we do some algebra on the condition in this while loop,
    // ie "quotient_hi * divisor_lo > (partial_remainder_hi << 32) | numerator_mid"
    // we end up getting "quotient_hi * shifted_divisor < (numerator_midbits << 32) | numerator_mid". remember that the right side of the inequality sign is complete_first_numerator from above.
    // which deminstrates that we're decrementing the quotient until the quotient multipled by the complete divisor is less than the complete numerator
    while partial_remainder_hi <= U64_MAX && quotient_hi * divisor_lo > (partial_remainder_hi << 64) | inner_numerator_mid {
        quotient_hi -= 1;
        partial_remainder_hi += divisor_hi;
    }

    // step 2: Divide the bottom part of the numerator. We're going to have the same problem as step 1, where we want the numerator to be a 192-bit number, so again we're going to split it into 2 substeps
	// the full numeratoe for step 3 will be:
	// complete_second_numerator = (first_division_remainder << 64) | inner_numerator_lo

    // step 2a: divide the upper part of the lower numerator by the upper part of the divisor
    // To get the numerator, complate the calculation of the full remainder by subtracing the quotient times the lower bits of the divisor
    // TODO: a warpping subtract is necessary here. why does this work, and why is it necessary?
    let full_remainder_hi = ((partial_remainder_hi << 64) | inner_numerator_mid).wrapping_sub(quotient_hi * divisor_lo);

    let mut quotient_lo = core::cmp::min(divide_128(full_remainder_hi, divisor_hi), U64_MAX);
    let mut partial_remainder_lo = full_remainder_hi - quotient_lo * divisor_hi;

    // step 2b: just like step 1b, decrement the final quotient until it's correctr when accounting for the full divisor
    while partial_remainder_lo <= U64_MAX && quotient_lo * divisor_lo > (partial_remainder_lo << 64) | inner_numerator_lo {
        quotient_lo -= 1;
        partial_remainder_lo += full_remainder_hi;
    }

    // We now have our separate quotients, now we just have to add them together
    (quotient_hi << 64) | quotient_lo
}

mod unit_tests {
	#[test]
	fn test_divide_128() {
		// Test divisors smaller than 64 bits
		for divisor in 1..100 {
			// test numerators whose upper 64 bits are less than the divisor
			for numerator_upper in 0..divisor {
				let numerator = (numerator_upper << 64) | (1 << 32);

				let expected_quotient = numerator / divisor;
		        let actual_quotient = super::divide_128(numerator, divisor);

		        assert_eq!(expected_quotient, actual_quotient, "wrong quotient for {}/{}", numerator, divisor);
			}

			//test numerators whose upper64 bits are >= the divisor
			for numerator_upper in divisor..105 {
				let numerator = (numerator_upper << 64) | (1 << 32);

				let expected_quotient = numerator / divisor;
		        let actual_quotient = super::divide_128(numerator, divisor);

		        assert_eq!(expected_quotient, actual_quotient, "wrong quotient for {}/{}", numerator, divisor);
			}
		}
		// test divisors greater than 64 bits
		for divisor_upper in 0..100 {
			for numerator in core::u128::MAX-100..=core::u128::MAX {
				let divisor = (divisor_upper << 64) | (1 << 32);

				let expected_quotient = numerator / divisor;
		        let actual_quotient = super::divide_128(numerator, divisor);

		        assert_eq!(expected_quotient, actual_quotient, "wrong quotient for {}/{}", numerator, divisor);
		    }
		}

		// overflow test
		for divisor in core::u128::MAX-5..core::u128::MAX {
			for numerator in core::u128::MAX-5..=core::u128::MAX {
				let expected_quotient = numerator / divisor;
		        let actual_quotient = super::divide_128(numerator, divisor);

		        assert_eq!(expected_quotient, actual_quotient, "wrong quotient for {}/{}", numerator, divisor);
		    }
		}
	}

	#[test]
	fn test_divide_128_by_64() {
		for divisor in core::u64::MAX..=core::u64::MAX {
			let divisor_128 = core::u64::MAX as u128;

			let numerator = divisor_128 * divisor_128 + (divisor_128 - 1);
			//for numerator in core::u128::MAX - 10..core::u128::MAX {
		        let expected_quotient = numerator / divisor as u128;
		        assert!(expected_quotient == core::u64::MAX as u128);

		        let actual_quotient = super::divide_128_by_64_helper(numerator as u128, divisor);

		        

		        let expected_upper = (expected_quotient >> 32) as u64;
		        let expected_lower = expected_quotient as u32 as u64;
		        let actual_upper = (actual_quotient >> 32) as u64;
		        let actual_lower = actual_quotient as u32 as u64;

		        assert_eq!(expected_upper, actual_upper, "wrong quotient for {}/{}", numerator, divisor);
		        assert_eq!(expected_lower, actual_lower, "wrong quotient for {}/{}", numerator, divisor);
		    //}
	    }
	}
}
