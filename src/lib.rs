#![no_std]

#[cfg(feature = "std")]
macro_rules! global_alloc {
    ([$($t:tt)*] AllocRef $($rest:tt)*) => {
        global_alloc! { [ $($t)* AllocRef = Global ] $($rest)* }
    };
    ([$($t:tt)*] $first:tt $($rest:tt)*) => {
        global_alloc! { [ $($t)* $first ] $($rest)* }
    };
    ([$($t:tt)*]) => {
        $($t)*
    };
    ($($t:tt)*) => {
        global_alloc! { [] $($t)* }
    };
}
#[cfg(not(feature = "std"))]
macro_rules! global_alloc {
    ($($t:tt)*) => { $($t)* };
}

#[cfg(feature = "std")]
#[doc(hidden)]
#[macro_export]
macro_rules! test_using_global {
    ($($t:tt)*) => { $($t)* };
}

#[cfg(not(feature = "std"))]
#[doc(hidden)]
#[macro_export]
macro_rules! test_using_global {
    ($($t:tt)*) => { fn main() {} };
}

#[path = "libcore/alloc.rs"]
mod core_alloc;
#[path = "libstd/alloc.rs"]
mod std_alloc;
#[path = "liballoc/boxed.rs"]
pub mod boxed;
#[path = "liballoc/collections/mod.rs"]
pub mod collections;
#[path = "liballoc/raw_vec.rs"]
pub mod raw_vec;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
mod global {
    use core::ptr::NonNull;
    use crate::core_alloc::{AllocErr, Layout};

    use std::alloc::{alloc, alloc_zeroed, dealloc, realloc};

    #[derive(Copy, Clone, Default, Debug)]
    pub struct Global;

    unsafe impl crate::core_alloc::AllocRef for Global {
        fn alloc(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), AllocErr> {
            NonNull::new(unsafe { alloc(layout.into()) })
                .ok_or(AllocErr)
                .map(|p| (p, layout.size()))
        }

        unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
            dealloc(ptr.as_ptr(), layout.into())
        }

        unsafe fn realloc(
            &mut self,
            ptr: NonNull<u8>,
            layout: Layout,
            new_size: usize,
        ) -> Result<(NonNull<u8>, usize), AllocErr> {
            NonNull::new(realloc(ptr.as_ptr(), layout.into(), new_size))
                .ok_or(AllocErr)
                .map(|p| (p, new_size))
        }

        fn alloc_zeroed(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), AllocErr> {
            NonNull::new(unsafe { alloc_zeroed(layout.into()) })
                .ok_or(AllocErr)
                .map(|p| (p, layout.size()))
        }
    }
}

pub mod alloc {
    pub use crate::core_alloc::*;
    pub use crate::std_alloc::rust_oom as handle_alloc_error;
    pub use crate::std_alloc::{set_alloc_error_hook, take_alloc_error_hook};

    #[cfg(feature = "std")]
    pub use crate::global::Global;
}

pub use crate::alloc::*;
pub use crate::boxed::*;
pub use crate::raw_vec::*;

use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

pub(crate) struct Unique<T: ?Sized> {
    pointer: NonNull<T>,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send + ?Sized> Send for Unique<T> {}
unsafe impl<T: Sync + ?Sized> Sync for Unique<T> {}

impl<T: Sized> Unique<T> {
    pub const fn empty() -> Self {
        unsafe {
            Unique::new_unchecked(mem::align_of::<T>() as *mut T)
        }
    }
}

impl<T: ?Sized> Unique<T> {
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        Unique { pointer: NonNull::new_unchecked(ptr), _marker: PhantomData }
    }

    pub fn as_ptr(self) -> *mut T {
        self.pointer.as_ptr()
    }

    pub unsafe fn as_ref(&self) -> &T {
        self.pointer.as_ref()
    }

    pub unsafe fn as_mut(&mut self) -> &mut T {
        self.pointer.as_mut()
    }
}

impl<T: ?Sized> Clone for Unique<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Unique<T> { }

impl<'a, T: ?Sized> From<NonNull<T>> for Unique<T> {
    fn from(p: NonNull<T>) -> Self {
        Unique { pointer: p, _marker: PhantomData }
    }
}

impl<T: ?Sized> From<Unique<T>> for NonNull<T> {
    #[inline]
    fn from(unique: Unique<T>) -> Self {
        unsafe { NonNull::new_unchecked(unique.as_ptr()) }
    }
}
