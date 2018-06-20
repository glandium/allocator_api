#![no_std]
#![cfg_attr(not(feature = "nonnull_cast"), allow(unstable_name_collision))]
#![cfg_attr(all(feature = "std", not(feature = "global_alloc")), feature(allocator_api))]

#[cfg(feature = "std")]
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
#[path = "liballoc/raw_vec.rs"]
pub mod raw_vec;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
mod global {
    use core::ptr::NonNull;
    use core_alloc::{AllocErr, Layout};

    #[cfg(feature = "global_alloc")]
    use core::alloc::Layout as CoreLayout;
    #[cfg(feature = "global_alloc")]
    use std::alloc::{alloc, alloc_zeroed, dealloc, realloc};

    #[cfg(not(feature = "global_alloc"))]
    use core::heap::{Alloc, Layout as CoreLayout};
    #[cfg(not(any(feature = "global_alloc", feature = "global_alloc27")))]
    use std::heap::Heap;
    #[cfg(feature = "global_alloc27")]
    use std::heap::Global as Heap;

    #[derive(Copy, Clone, Default, Debug)]
    pub struct Global;

    impl From<Layout> for CoreLayout {
        fn from(l: Layout) -> Self {
            unsafe { Self::from_size_align_unchecked(l.size(), l.align()) }
        }
    }

    #[cfg(feature = "global_alloc")]
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

    #[cfg(not(feature = "global_alloc"))]
    impl From<::core::heap::AllocErr> for AllocErr {
        fn from(_: ::core::heap::AllocErr) -> Self {
            AllocErr
        }
    }

    #[cfg(not(any(feature = "global_alloc", feature = "global_alloc27")))]
    unsafe impl ::core_alloc::Alloc for Global {
        unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
            NonNull::new(Heap.alloc(layout.into())?).ok_or(AllocErr)
        }

        unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
            Heap.dealloc(ptr.as_ptr(), layout.into())
        }

        unsafe fn realloc(&mut self,
                          ptr: NonNull<u8>,
                          layout: Layout,
                          new_size: usize) -> Result<NonNull<u8>, AllocErr> {
            NonNull::new(Heap.realloc(ptr.as_ptr(), layout.into(), CoreLayout::from_size_align_unchecked(new_size, layout.align()))?).ok_or(AllocErr)
        }

        unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
            NonNull::new(Heap.alloc_zeroed(layout.into())?).ok_or(AllocErr)
        }
    }

    #[cfg(feature = "global_alloc27")]
    unsafe impl ::core_alloc::Alloc for Global {
        unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
            Ok(Heap.alloc(layout.into())?.cast())
        }

        unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
            Heap.dealloc(NonNull::new_unchecked(ptr.as_ptr() as *mut ::core::alloc::Opaque), layout.into())
        }

        unsafe fn realloc(&mut self,
                          ptr: NonNull<u8>,
                          layout: Layout,
                          new_size: usize) -> Result<NonNull<u8>, AllocErr> {
            Ok(Heap.realloc(NonNull::new_unchecked(ptr.as_ptr() as *mut ::core::alloc::Opaque), layout.into(), new_size)?.cast())
        }

        unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
            Ok(Heap.alloc_zeroed(layout.into())?.cast())
        }
    }

}

pub mod alloc {
    pub use core_alloc::*;
    pub use std_alloc::rust_oom as handle_alloc_error;
    pub use std_alloc::{set_alloc_error_hook, take_alloc_error_hook};

    #[cfg(feature = "std")]
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
