#![no_std]

pub mod allocator;
pub mod boxed;
pub mod raw_vec;

pub use allocator::*;
pub use boxed::*;
pub use raw_vec::*;

use core::ptr::NonNull;

pub trait NonNullCast {
    fn cast_<U>(self) -> NonNull<U>;
    fn as_opaque_(self) -> NonNull<Opaque>;
}

impl<T: ?Sized> NonNullCast for NonNull<T> {
    fn cast_<U>(self) -> NonNull<U> {
        unsafe {
            NonNull::new_unchecked(self.as_ptr() as *mut U)
        }
    }

    fn as_opaque_(self) -> NonNull<Opaque> {
        self.cast_()
    }
}
