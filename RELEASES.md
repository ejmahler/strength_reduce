# Release 0.2.1 (2019-01-04)

### Fixes

- Fixed a class of bugs for certain divisors with very large numerators, where the returned quotient was off by one.

# Release 0.2.0 (2019-01-03)

### Breaking Changes

- `strength_reduce` is now marked `#[!no_std]`

# Release 0.1.1 (2019-01-03)

 - Added the readme to cargo.tom, so that it can be rendered directly from crates.io

# Release 0.1.0 (2019-01-03)

 - Initial release. Support for computing strength-reduced division and modulo for unsigned integers:
 - `u8`: `StrengthReducedU8`
 - `u16`: `StrengthReducedU16`
 - `u32`: `StrengthReducedU32`
 - `u64`: `StrengthReducedU64`
 - `usize`: `StrengthReducedUsize`
