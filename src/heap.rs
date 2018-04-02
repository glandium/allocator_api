// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(feature = "unstable-rust")]
pub use alloc::heap::Heap;

#[cfg(not(feature = "unstable-rust"))]
mod heap {
    use ::allocator::{Alloc, AllocErr, Layout};
    use std::{mem, ptr};
    use std::vec::Vec;

    #[derive(Copy, Clone, Default, Debug)]
    pub struct Heap;

    #[inline(always)]
    fn alloc_with<T, F: FnOnce() -> Vec<T>>(f: F) -> *mut u8 {
        let mut v = f();
        let raw: *mut T = v.as_mut_ptr();
        mem::forget(v);
        raw as *mut u8
    }

    macro_rules! aligned_vec_zero {
        ($t:ty, $size:expr, $align:expr) => {{
            const_assert_eq!(mem::align_of::<$t>(), $align);
            vec![0 as $t; $size]
        }}
    }
    #[inline(always)]
    fn alloc_zeroed<T>(size: usize) -> *mut u8 {
        match mem::size_of::<T>() {
            // vec! has an optimization that uses calloc when using
            // vec![0; n], so use that when we can.
            1 => alloc_with(|| aligned_vec_zero!(u8, size, 1)),
            2 => alloc_with(|| aligned_vec_zero!(u16, size, 2)),
            #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
            4 => alloc_with(|| aligned_vec_zero!(u32, size, 4)),
            #[cfg(target_pointer_width = "64")]
            8 => alloc_with(|| aligned_vec_zero!(u64, size, 8)),
            n => alloc_with(|| {
                let mut v = Vec::<T>::with_capacity(size);
                unsafe { ptr::write_bytes(v.as_mut_ptr(), 0, size * n) };
                v
            }),
        }
    }

    macro_rules! heap_impl {
        ($($name:ident: $align:tt),+ $(,)*) => {
            $( #[repr(align($align))]
            #[derive(Clone)]
            struct $name([u8; $align]); )+

            unsafe impl Alloc for Heap {
                #[inline(always)]
                unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
                    let size = (layout.size() + layout.align() - 1) / layout.align();
                    match layout.align() {
                        $($align => Ok(alloc_with(|| {
                            Vec::<$name>::with_capacity(size)
                        })),)+
                        _ => Err(AllocErr::Unsupported{ details: "" }),
                    }
                }

                #[inline(always)]
                unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
                    mem::drop(Vec::from_raw_parts(ptr, layout.size(), layout.size()));
                }

                #[inline(always)]
                unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
                    let size = (layout.size() + layout.align() - 1) / layout.align();
                    match layout.align() {
                        $($align => Ok(alloc_zeroed::<$name>(size)),)+
                        _ => Err(AllocErr::Unsupported{ details: "" }),
                    }
                }
            }
        };
    }

    heap_impl! {
        Aligned1: 1,
        Aligned2: 2,
        Aligned4: 4,
        Aligned8: 8,
        Aligned16: 16,
        Aligned32: 32,
        Aligned64: 64,
        Aligned128: 128,
        Aligned256: 256,
        Aligned512: 512,
        Aligned1K: 1024,
        Aligned2K: 2048,
        Aligned4K: 4096,
        Aligned8K: 8192,
        Aligned16K: 16384,
        Aligned32K: 32768,
        Aligned64K: 65536,
        Aligned128K: 131072,
        Aligned256K: 262144,
        Aligned512K: 524288,
        Aligned1M: 1048576,
        Aligned2M: 2097152,
        Aligned4M: 4194304,
        Aligned8M: 8388608,
        Aligned16M: 16777216,
        Aligned32M: 33554432,
        Aligned64M: 67108864,
        Aligned128M: 134217728,
        Aligned256M: 268435456,
        Aligned512M: 536870912,
    }
}

#[cfg(not(feature = "unstable-rust"))]
pub use self::heap::Heap;
