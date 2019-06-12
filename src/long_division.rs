const U32_MAX: u64 = core::u32::MAX as u64;

// divides core::u128::MAX by a 64-bit divisor, much faster than upcasting the divisor to 128 bits and applying standard division
pub(crate) fn divide_128_max(divisor: u64) -> u128 {
	if divisor <= U32_MAX {
		return divide_128_small_max(divisor as u32);
	}

    let shift_size = divisor.leading_zeros();
	let shifted_divisor = divisor << shift_size;
	let divisor_hi = shifted_divisor >> 32;
    let divisor_lo = shifted_divisor as u32 as u64;

	// We're going to do a long division, composed of 3 separate divisions
	// step 1: divide the uppermost bits of the numerator by the divisor
    let quotient_highbits = core::u64::MAX / divisor;
    let remainder_highbits = core::u64::MAX - quotient_highbits * divisor;

    // step 2: divide another big chunk of the numerator by the divisor
    // IDEALLY, we would divide (remainder_highbits << shift_size + 64) | core::u64::MAX by shifted_divisor, but that would require a 128-bit numerator, which is the whole thing we're trying to avoid
    // so instead we're going to split the second division into two sub-phases. in 2a, we dividing numerator_midbits by divior_hi, and then in 2b we decrement the quotient to account for the lower bits of the divisor

    // keep in mind that for all of step 2, the full numerator we're using will be
    // complete_middle_numerator = (numerator_midbits << 32) | U32_MAX

    // step 2a: divide the upper part of the middle numerator by the upper part of the divisor
    let numerator_midbits = if shift_size == 0 { remainder_highbits } else { (remainder_highbits << shift_size) | (core::u64::MAX >> (64 - shift_size)) };
    let mut quotient_midbits = core::cmp::min(numerator_midbits / divisor_hi, U32_MAX);
    let mut partial_remainder_midbits = numerator_midbits - quotient_midbits * divisor_hi;

    // step 2b: we know sort of what the quotient is, but it's slightly too large because it doesn't account for the lower bits of the divisor. so decrement the quotient until it fits
    // note that if we do some algebra on the condition in this while loop,
    // ie "quotient_midbits * divisor_lo > (partial_remainder_midbits << 32) | U32_MAX"
    // we end up getting "quotient_midbits * shifted_divisor < (numerator_midbits << 32) | U32_MAX". remember that the right side of the inequality sign is complete_middle_numerator from above.
    // which deminstrates that we're decrementing the quotient until the quotient multipled by the complete divisor is less than the complete numerator
    while partial_remainder_midbits <= U32_MAX && quotient_midbits * divisor_lo > (partial_remainder_midbits << 32) | U32_MAX {
        quotient_midbits -= 1;
        partial_remainder_midbits += divisor_hi;
    }
    // step 3: Divide the bottom part of the numerator. We're oging to have the same problem as step 2, where we want the numerator to be a 96-bit number, so again we're going to split it into 2 substeps
	// the full numeratoe for step 3 will be:
	// complete_bottom_numerator = (numerator_lowbits << 32) | (core::u32::MAX << shift_size) as u64

    // step 3a: divide the upper part of the lower numerator by the upper part of the divisor
    // To get the numerator, complate the calculation of the full remainder by subtracing the quotient times the lower bits of the divisor
    // TODO: a warpping subtract is necessary here. why does this work, and why is it necessary?
    let numerator_lowbits = ((partial_remainder_midbits << 32) | U32_MAX).wrapping_sub(quotient_midbits * divisor_lo);

    let mut quotient_lowbits = core::cmp::min(numerator_lowbits / divisor_hi, U32_MAX);
    let mut remainder_lowbits = numerator_lowbits - quotient_lowbits * divisor_hi;

    // phase 3b: just like step 2b, decrement the final quotient until it's correctr when accounting for the full divisor
    let final_numerator_chunk = (core::u32::MAX << shift_size) as u64;
    while remainder_lowbits <= U32_MAX && quotient_lowbits * divisor_lo > (remainder_lowbits << 32) | final_numerator_chunk {
        quotient_lowbits -= 1;
        remainder_lowbits += divisor_hi;
    }

    // We now have all our separate quotients, now we just have to add them together
    let quotiont_lowerhalf = (quotient_midbits << 32) | quotient_lowbits;
    ((quotient_highbits as u128) << 64) | quotiont_lowerhalf as u128
}

// this is a version of divide_128_max optimized around the fact that shift_size would always be 32 or more,
// therefore divisor_lo will always be 0,
// therefore the while loops to fix up the quotient aren't necessary
// benchmarking shows that this shaves 10-20% off divisions that fit inside a u32. It seems like most divisors will fit inside a u32, so this will be the most common path
fn divide_128_small_max(small_divisor: u32) -> u128 {
	let divisor = small_divisor as u64;
	let shift_size = small_divisor.leading_zeros();
	let shifted_divisor = divisor << shift_size;

	// We're going to do a long division, composed of 3 separate divisions
	// step 1: divide the uppermost bits of the numerator by the divisor
    let quotient_highbits = core::u64::MAX / divisor;
    let remainder_highbits = core::u64::MAX - quotient_highbits * divisor;

    // step 2: the remainder of the first division, divided by a shifted version of the divisor
    // We also shift some more of the numerator in
    let numerator_midbits = (remainder_highbits << (shift_size + 32)) | (core::u64::MAX >> (32 - shift_size));
   
    let quotient_midbits = numerator_midbits / shifted_divisor;
    let remainder_midbits = numerator_midbits - quotient_midbits * shifted_divisor;

    // step 3: the remainder of the second divison, divided by the divisor one last time
    // We also shift in the final section of the original u128 numerator
    let numerator_lowbits = remainder_midbits.wrapping_shl(32) + (core::u32::MAX << shift_size) as u64;
    let quotient_lowbits = numerator_lowbits / shifted_divisor;

    // We now have all our separate quotients, now we just have to add them together
    let quotiont_lowerhalf = (quotient_midbits << 32) | quotient_lowbits;
    ((quotient_highbits as u128) << 64) | quotiont_lowerhalf as u128
}


mod unit_tests {
	#[test]
	fn test_max_division_128_large() {
		for divisor in 1..=1000 {
	        let expected_output = core::u128::MAX / divisor as u128;
	        let actual_output = super::divide_128_max(divisor);

	        assert_eq!(expected_output, actual_output);
	    }
	    for divisor in core::u32::MAX-1000..=core::u32::MAX {
	        let expected_output = core::u128::MAX / divisor as u128;
	        let actual_output = super::divide_128_max(divisor as u64);

	        assert_eq!(expected_output, actual_output);
	    }
		for divisor in core::u32::MAX as u64+1..=core::u32::MAX as u64+1000 {
	        let expected_output = core::u128::MAX / divisor as u128;
	        let actual_output = super::divide_128_max(divisor);

	        assert_eq!(expected_output, actual_output);
	    }
	    for divisor in core::u64::MAX-1000..=core::u64::MAX {
	        let expected_output = core::u128::MAX / divisor as u128;
	        let actual_output = super::divide_128_max(divisor);

	        assert_eq!(expected_output, actual_output);
	    }
	}

	#[test]
	fn test_max_division_128_small() {
		for divisor in 1..=1000 {
	        let expected_output = core::u128::MAX / divisor as u128;
	        let actual_output = super::divide_128_small_max(divisor);

	        assert_eq!(expected_output, actual_output);
	    }
	    for divisor in core::u32::MAX-1000..=core::u32::MAX {
	        let expected_output = core::u128::MAX / divisor as u128;
	        let actual_output = super::divide_128_small_max(divisor);

	        assert_eq!(expected_output, actual_output);
	    }
	}
}
