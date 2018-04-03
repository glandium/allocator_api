# allocator_api

This is a copy of the unstable allocator_api
(https://github.com/rust-lang/rust/issues/32838) and of parts of the unstable
alloc feature.

Usable with stable rust, but requires 1.25.

## Differences with nightly rust

The code was copied from the rust repository as of
8dd24c8ed4ac3e48068408fa21d491d7ffe45295, with #[stable] and #[unstable]
annotations removed.

In the allocator module (corresponding to core::heap), the `oom` function calls
`panic!` instead of `core::intrinsics::abort`, which is not stable. This
presumes `panic!` doesn't require memory allocation.

In the raw_vec module (corresponding to alloc::raw_vec), `RawVec` uses
`NonNull` instead of `Unique`.

In the boxed module (corresponding to alloc::boxed), the `Box` type is
augmented such that it is associated with an allocator, similarly to `RawVec`.
Its API is consequently slightly different from `std::boxed::Box` (e.g.
`from_raw` is replaced with `from_raw_in`). The same (stable) features as
`std::boxed::Box` are implemented, except for `downcast` for `Box<Any>` and
`Box<Any + Send>`, and `Box<str>` functions. Like for `RawVec`, the type
relies on `NonNull` rather than `Unique`.

Caveat: the types provided in this crate cannot be used where the corresponding
types from `std`/`alloc` are expected. Few APIs should be taking those types
directly as input, though.
