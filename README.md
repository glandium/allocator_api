# allocator_api

This is a copy of the unstable allocator_api
(https://github.com/rust-lang/rust/issues/32838) and of parts of the unstable
alloc feature.

Usable with stable rust, but requires 1.61.

## Differences with nightly rust

The code was copied from the rust code as of 1.36.0
with #[stable] annotations and #[unstable] implementations removed.

In the alloc module (corresponding to parts of both core::alloc and
std::alloc), the `oom` function infinitely loops instead of calling
`core::intrinsics::abort`, which is not stable. Users of this crate should use
`set_oom_hook` to set their own oom function that aborts in the right way (in
non-no_std cases, one can use `process::abort()`).

In the raw_vec module (corresponding to alloc::raw_vec), `RawVec` uses
`NonNull` instead of `Unique`.

In the boxed module (corresponding to alloc::boxed), the `Box` type is
augmented such that it is associated with an allocator, similarly to `RawVec`.
Its API is consequently slightly different from `std::boxed::Box` (e.g.
`from_raw` is replaced with `from_raw_in`). The same (stable) features as
`std::boxed::Box` are implemented, except for `downcast` for `Box<Any>` and
`Box<Any + Send>`. Like for `RawVec`, the type relies on `NonNull` rather than
`Unique`.

Caveat: the types provided in this crate cannot be used where the corresponding
types from `std`/`alloc` are expected. Few APIs should be taking those types
directly as input, though.
