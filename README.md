# strength_reduce
[![crate](https://img.shields.io/crates/v/strength_reduce.svg)](https://crates.io/crates/strength_reduce)
[![license](https://img.shields.io/crates/l/strength_reduce.svg)](https://crates.io/crates/strength_reduce)
[![documentation](https://docs.rs/strength_reduce/badge.svg)](https://docs.rs/strength_reduce/)
![minimum rustc 1.26](https://img.shields.io/badge/rustc-1.26+-red.svg)

Faster integer division and modulus operations.

`strength_reduce` uses arithmetic strength reduction to transform divisions into multiplications and shifts.
When the divisor is not known at compile time, this yields a 5x-10x speedup for integer division and modulo operations,
with a small amortized setup cost.

This library is intended for hot loops like the example below, where a division is repeated hundreds of times in a loop, but the divisor remains unchanged. There is a setup cost associated with creating stength-reduced division instances, so using strength-reduced division for 1-2 divisions is not worth the setup cost. The break-even point differs by use-case, but appears to typically be around 5-10 for u8-u32, and 30-40 for u64.

`strength_reduce` is `#![no_std]`

See the [API Documentation](https://docs.rs/strength_reduce/) for more details.

## Example
```rust
use strength_reduce::StrengthReducedU64;

let mut my_array: Vec<u64> = (0..500).collect();
let divisor = 3;
let modulo = 14;

// slow naive division and modulo
for element in &mut my_array {
    *element = (*element / divisor) % modulo;
}

// fast strength-reduced division and modulo
let reduced_divisor = StrengthReducedU64::new(divisor);
let reduced_modulo = StrengthReducedU64::new(modulo);
for element in &mut my_array {
    *element = (*element / reduced_divisor) % reduced_modulo;
}
```

## Testing

`strength_reduce` uses `proptest` to generate test cases. In addition, the `u8` and `u16` problem spaces are small enough that we can exhaustively test every possible combination of numerator and divisor.
However, the `u16` exhaustive test takes several minutes to run, so it is marked `#[ignore]`. Before submitting pull requests, please test with `cargo test -- --ignored` at least once.

## Compatibility

The `strength_reduce` crate requires rustc 1.26 or greater.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

