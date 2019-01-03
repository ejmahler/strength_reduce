#![feature(test)]
extern crate test;
extern crate strength_reduce;

macro_rules! reduced_division_bench {
    ($standard_bench_name:ident, $reduced_bench_name:ident, $struct_name:ident, $primitive_type:ident) => (
        #[bench]
		fn $standard_bench_name(b: &mut test::Bencher) {
			let width: $primitive_type = 200;
			let height: $primitive_type = 90;
		    b.iter(|| { 
		    	let inner_height = test::black_box(height);

		    	let mut sum = 0;
		    	for y in 0..height {
					for x in 0..width {
						sum += (x + y) / inner_height;
					}
				}
		    	test::black_box(sum);
		     });
		}

		#[bench]
		fn $reduced_bench_name(b: &mut test::Bencher) {
			let width: $primitive_type = 200;
			let height: $primitive_type = 90;

			let reduced_height = strength_reduce::$struct_name::new(height);
		    b.iter(|| { 
				let mut sum = 0;
				for y in 0..height {
					for x in 0..width {
						sum += (x + y) / reduced_height;
					}
				}
				test::black_box(sum);
			});
		}
    )
}

reduced_division_bench!(bench_standard_division_u08, bench_reduced_division_u08, StrengthReducedU8, u8);
reduced_division_bench!(bench_standard_division_u16, bench_reduced_division_u16, StrengthReducedU16, u16);
reduced_division_bench!(bench_standard_division_u32, bench_reduced_division_u32, StrengthReducedU32, u32);
reduced_division_bench!(bench_standard_division_u64, bench_reduced_division_u64, StrengthReducedU64, u64);

macro_rules! reduced_mod_bench {
    ($standard_bench_name:ident, $reduced_bench_name:ident, $struct_name:ident, $primitive_type:ident) => (
        #[bench]
		fn $standard_bench_name(b: &mut test::Bencher) {
			let width: $primitive_type = 200;
			let height: $primitive_type = 90;
		    b.iter(|| { 
		    	let inner_height = test::black_box(height);

		    	let mut sum = 0;
		    	for y in 0..height {
			    	for x in 0..width {
			    		sum += (x * width + y) % inner_height;
			    	}
			    }
		    	test::black_box(sum);
		     });
		}

		#[bench]
		fn $reduced_bench_name(b: &mut test::Bencher) {
			let width: $primitive_type = 200;
			let height: $primitive_type = 90;

			let reduced_height = strength_reduce::$struct_name::new(height);
		    b.iter(|| { 
				let mut sum = 0;
		    	for y in 0..height {
			    	for x in 0..width {
			    		sum += (x * width + y) % reduced_height;
			    	}
			    }
		    	test::black_box(sum);
			});
		}
    )
}

reduced_mod_bench!(bench_standard_modulo_u08, bench_reduced_modulo_u08, StrengthReducedU8, u8);
reduced_mod_bench!(bench_standard_modulo_u16, bench_reduced_modulo_u16, StrengthReducedU16, u16);
reduced_mod_bench!(bench_standard_modulo_u32, bench_reduced_modulo_u32, StrengthReducedU32, u32);
reduced_mod_bench!(bench_standard_modulo_u64, bench_reduced_modulo_u64, StrengthReducedU64, u64);