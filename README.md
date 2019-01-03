# strength_reduce
[![crate](https://img.shields.io/crates/v/strength_reduce.svg)](https://crates.io/crates/strength_reduce)
[![license](https://img.shields.io/crates/l/strength_reduce.svg)](https://crates.io/crates/strength_reduce)
[![documentation](https://docs.rs/strength_reduce/badge.svg)](https://docs.rs/strength_reduce/)
![minimum rustc 1.26](https://img.shields.io/badge/rustc-1.26+-red.svg)

Faster integer division and modulus operations.

`strength_reduce` uses arithmetic strength reduction to transform divisions into multiplications and shifts. This yields a 5x-10x speedup for integer division and modulo operations, with a small amortized setup cost.

Although this library can speed up any division or modulo operation, it's intended for hot loops like the example below, where a division is repeated hundreds of times in a loop, but the divisor remains unchanged.

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
## Compatibility

The `strength_reduce` crate requires rustc 1.26 or greater.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

