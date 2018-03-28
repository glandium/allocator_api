# allocator_api

This is a copy of the unstable allocator_api
(https://github.com/rust-lang/rust/issues/32838)

Usable with stable rust, but requires 1.25.

## Differences with nightly rust

The code was copied from src/liballoc/allocator.rs as of
92bfcd2b192e59d12d64acf6f46c1897a3273b3e, with #[unstable] annotations
removed, and the `oom` function calling `panic!` instead of
`core::intrinsics::abort`, which is not stable. This presumes `panic!`
doesn't require memory allocation.
