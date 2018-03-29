// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod their {
    pub use alloc::heap::Heap;
    pub use alloc::allocator::{AllocErr, CannotReallocInPlace, Excess, Layout};
}
use alloc::allocator::Alloc;
use super::allocator as our;

#[derive(Copy, Clone, Default, Debug)]
pub struct Heap;

unsafe impl our::Alloc for Heap {
    #[inline]
    unsafe fn alloc(&mut self, layout: our::Layout) -> Result<*mut u8, our::AllocErr> {
        Ok(their::Heap.alloc(layout.into())?)
    }

    #[inline]
    #[cold]
    fn oom(&mut self, err: our::AllocErr) -> ! {
        their::Heap.oom(err.into())
    }

    #[inline]
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: our::Layout) {
        their::Heap.dealloc(ptr, layout.into())
    }

    #[inline]
    fn usable_size(&self, layout: &our::Layout) -> (usize, usize) {
        let layout = unsafe {
            their::Layout::from_size_align_unchecked(layout.size(), layout.align())
        };
        their::Heap.usable_size(&layout)
    }

    #[inline]
    unsafe fn realloc(&mut self,
                      ptr: *mut u8,
                      layout: our::Layout,
                      new_layout: our::Layout)
                      -> Result<*mut u8, our::AllocErr>
    {
        Ok(their::Heap.realloc(ptr, layout.into(), new_layout.into())?)
    }

    #[inline]
    unsafe fn alloc_zeroed(&mut self, layout: our::Layout) -> Result<*mut u8, our::AllocErr> {
        Ok(their::Heap.alloc_zeroed(layout.into())?)
    }

    #[inline]
    unsafe fn alloc_excess(&mut self, layout: our::Layout) -> Result<our::Excess, our::AllocErr> {
        Ok(their::Heap.alloc_excess(layout.into())?.into())
    }

    #[inline]
    unsafe fn realloc_excess(&mut self,
                             ptr: *mut u8,
                             layout: our::Layout,
                             new_layout: our::Layout) -> Result<our::Excess, our::AllocErr> {
        Ok(their::Heap.realloc_excess(ptr, layout.into(), new_layout.into())?.into())
    }

    #[inline]
    unsafe fn grow_in_place(&mut self,
                            ptr: *mut u8,
                            layout: our::Layout,
                            new_layout: our::Layout)
                            -> Result<(), our::CannotReallocInPlace>
    {
        Ok(their::Heap.grow_in_place(ptr, layout.into(), new_layout.into())?)
    }

    #[inline]
    unsafe fn shrink_in_place(&mut self,
                              ptr: *mut u8,
                              layout: our::Layout,
                              new_layout: our::Layout) -> Result<(), our::CannotReallocInPlace>
    {
        Ok(their::Heap.shrink_in_place(ptr, layout.into(), new_layout.into())?)
    }
}

impl From<our::Layout> for their::Layout {
    fn from(l: our::Layout) -> Self {
        unsafe { their::Layout::from_size_align_unchecked(l.size(), l.align()) }
    }
}

impl From<their::Layout> for our::Layout {
    fn from(l: their::Layout) -> Self {
        unsafe { our::Layout::from_size_align_unchecked(l.size(), l.align()) }
    }
}

impl From<their::AllocErr> for our::AllocErr {
    fn from(e: their::AllocErr) -> Self {
        match e {
            their::AllocErr::Exhausted { request: r } => our::AllocErr::Exhausted { request: r.into() },
            their::AllocErr::Unsupported { details: d } => our::AllocErr::Unsupported { details: d },
        }
    }
}

impl From<our::AllocErr> for their::AllocErr {
    fn from(e: our::AllocErr) -> Self {
        match e {
            our::AllocErr::Exhausted { request: r } => their::AllocErr::Exhausted { request: r.into() },
            our::AllocErr::Unsupported { details: d } => their::AllocErr::Unsupported { details: d },
        }
    }
}

impl From<their::Excess> for our::Excess {
    fn from(e: their::Excess) -> Self {
        our::Excess(e.0, e.1)
    }
}

impl From<their::CannotReallocInPlace> for our::CannotReallocInPlace {
    fn from(_: their::CannotReallocInPlace) -> Self {
        our::CannotReallocInPlace
    }
}
