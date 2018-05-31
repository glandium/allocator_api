#![no_std]
#![allow(unstable_name_collision)]

#[path = "libcore/alloc.rs"]
mod core_alloc;
#[path = "libstd/alloc.rs"]
mod std_alloc;
#[path = "liballoc/boxed.rs"]
pub mod boxed;
#[path = "liballoc/raw_vec.rs"]
pub mod raw_vec;

pub mod alloc {
    pub use core_alloc::*;
    pub use std_alloc::rust_oom as oom;
    pub use std_alloc::{set_oom_hook, take_oom_hook};
}

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
