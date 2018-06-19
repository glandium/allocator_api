#![no_std]
#![cfg_attr(not(feature = "nonnull_cast"), allow(unstable_name_collision))]

#[cfg(feature = "global_alloc")]
macro_rules! global_alloc {
    ([$($t:tt)*] Alloc $($rest:tt)*) => {
        global_alloc! { [ $($t)* Alloc = Global ] $($rest)* }
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
#[cfg(not(feature = "global_alloc"))]
macro_rules! global_alloc {
    ($($t:tt)*) => { $($t)* };
}

#[path = "libcore/alloc.rs"]
mod core_alloc;
#[path = "libstd/alloc.rs"]
mod std_alloc;
#[path = "liballoc/boxed.rs"]
pub mod boxed;
#[path = "liballoc/raw_vec.rs"]
pub mod raw_vec;

#[cfg(feature = "global_alloc")]
extern crate std;

#[cfg(feature = "global_alloc")]
mod global {
    use core::ptr::NonNull;
    use core_alloc::{AllocErr, Layout};
    use std::alloc::{alloc, alloc_zeroed, dealloc, realloc};

    #[derive(Copy, Clone, Default, Debug)]
    pub struct Global;

    impl From<Layout> for ::core::alloc::Layout {
        fn from(l: Layout) -> Self {
            unsafe { Self::from_size_align_unchecked(l.size(), l.align()) }
        }
    }

    unsafe impl ::core_alloc::Alloc for Global {
        unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
            NonNull::new(alloc(layout.into())).ok_or(AllocErr)
        }

        unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
            dealloc(ptr.as_ptr(), layout.into())
        }

        unsafe fn realloc(&mut self,
                          ptr: NonNull<u8>,
                          layout: Layout,
                          new_size: usize) -> Result<NonNull<u8>, AllocErr> {
            NonNull::new(realloc(ptr.as_ptr(), layout.into(), new_size)).ok_or(AllocErr)
        }

        unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
            NonNull::new(alloc_zeroed(layout.into())).ok_or(AllocErr)
        }
    }
}

pub mod alloc {
    pub use core_alloc::*;
    pub use std_alloc::rust_oom as oom;
    pub use std_alloc::{set_oom_hook, take_oom_hook};

    #[cfg(feature = "global_alloc")]
    pub use global::Global;
}

pub use alloc::*;
pub use boxed::*;
pub use raw_vec::*;

#[cfg(not(feature = "nonnull_cast"))]
use core::ptr::NonNull;

/// Casting extensions to the `NonNull` type
///
/// This trait adds the [cast] method to the `NonNull` type, which is
/// only available starting from rust 1.27.
///
/// [cast]: https://doc.rust-lang.org/nightly/core/ptr/struct.NonNull.html#method.cast
#[cfg(not(feature = "nonnull_cast"))]
pub trait NonNullCast {
    fn cast<U>(self) -> NonNull<U>;
}

#[cfg(not(feature = "nonnull_cast"))]
impl<T: ?Sized> NonNullCast for NonNull<T> {
    fn cast<U>(self) -> NonNull<U> {
        unsafe {
            NonNull::new_unchecked(self.as_ptr() as *mut U)
        }
    }
}
