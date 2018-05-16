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
/// [cast]: https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html#method.cast
/// [as_opaque]: https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html#method.as_opaque
pub trait NonNullCast {
    fn cast<U>(self) -> NonNull<U>;
    fn as_opaque(self) -> NonNull<Opaque>;
}

impl<T: ?Sized> NonNullCast for NonNull<T> {
    fn cast<U>(self) -> NonNull<U> {
        unsafe {
            NonNull::new_unchecked(self.as_ptr() as *mut U)
        }
    }

    fn as_opaque(self) -> NonNull<Opaque> {
        self.cast()
    }
}
