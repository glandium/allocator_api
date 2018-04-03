# allocator_api

This is a copy of parts of the unstable allocator_api
(https://github.com/rust-lang/rust/issues/32838)

Usable with stable rust, but requires 1.25.

## Differences with nightly rust

The code was copied from src/liballoc as of
8dd24c8ed4ac3e48068408fa21d491d7ffe45295, with #[stable] and #[unstable]
annotations removed.

In the allocator module, the `oom` function calls `panic!` instead of
`core::intrinsics::abort`, which is not stable. This presumes `panic!`
doesn't require memory allocation.

In the raw_vec module, `RawVec` uses `NonNull` instead of `Unique`.

The `box` feature enables a `Box` type associated with a specific allocator,
which provides the same (stable) features as `std::boxed::box`, except for
`downcast` for `Box<Any>` and `Box<Any + Send>`, and `Box<str>` functions.
Like for `RawVec`, the type relies on `NonNull` rather than `Unique`.
