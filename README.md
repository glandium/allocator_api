# allocator_api

This is a copy of the unstable allocator_api
(https://github.com/rust-lang/rust/issues/32838) and of parts of the unstable
alloc feature.

Usable with stable rust, but requires 1.26.

## Differences with nightly rust

The code was copied from the rust repository as of
1caaafdce7871bc2816c9f42a14fd9262eda4037, with #[stable] and #[unstable]
annotations removed.

In the alloc module (corresponding to core::alloc), the `oom` function
infinitely loops instead of calling `core::intrinsics::abort`, which is not
stable. Implementations of the trait should override `oom` to handle the
situation more appropriately. The `Opaque` type is an empty enum instead of
an (not yet stable) extern type.

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
