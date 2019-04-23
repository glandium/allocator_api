#![no_std]

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

    use core::alloc::Layout as CoreLayout;
    use std::alloc::{alloc, alloc_zeroed, dealloc, realloc};

    #[derive(Copy, Clone, Default, Debug)]
    pub struct Global;

    impl From<Layout> for CoreLayout {
        fn from(l: Layout) -> Self {
            unsafe { Self::from_size_align_unchecked(l.size(), l.align()) }
        }
    }

    unsafe impl crate::core_alloc::Alloc for Global {
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
    pub use crate::core_alloc::*;
    pub use crate::std_alloc::rust_oom as handle_alloc_error;
    pub use crate::std_alloc::{set_alloc_error_hook, take_alloc_error_hook};

    #[cfg(feature = "std")]
    pub use crate::global::Global;
}

pub use crate::alloc::*;
pub use crate::boxed::*;
pub use crate::raw_vec::*;
