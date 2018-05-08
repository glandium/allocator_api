#![no_std]
#![cfg_attr(feature = "unstable_name_collision", allow(unstable_name_collision))]

pub mod alloc;
pub mod boxed;
pub mod raw_vec;

pub use alloc::*;
pub use boxed::*;
pub use raw_vec::*;

use core::ptr::NonNull;

/// Casting extensions to the `NonNull` type
///
/// This trait adds the [cast] and [as_opaque] methods to the `NonNull` type.
/// As of writing, [as_opaque] is still unstable, and [cast] only available
/// starting from rust 1.27.
///
/// Due to the conflict in name of the methods and to [PR #48552] not being
/// merged until rust 1.26, they are suffixed with an underscore.
///
/// [cast]: https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html#method.cast
/// [as_opaque]: https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html#method.as_opaque
/// [PR #48552]: https://github.com/rust-lang/rust/pull/48552
pub trait NonNullCast {
    fn cast_<U>(self) -> NonNull<U>;
    fn as_opaque(self) -> NonNull<Opaque>;
}

impl<T: ?Sized> NonNullCast for NonNull<T> {
    fn cast_<U>(self) -> NonNull<U> {
        unsafe {
            NonNull::new_unchecked(self.as_ptr() as *mut U)
        }
    }

    fn as_opaque(self) -> NonNull<Opaque> {
        self.cast_()
    }
}
