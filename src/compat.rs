// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod their {
    pub use alloc::allocator::{Alloc, AllocErr, CannotReallocInPlace, Excess, Layout};
}
use super::allocator as our;

unsafe impl<A: their::Alloc> our::Alloc for A {
    #[inline]
    unsafe fn alloc(&mut self, layout: our::Layout) -> Result<*mut u8, our::AllocErr> {
        Ok(their::Alloc::alloc(self, layout.into())?)
    }

    #[inline]
    #[cold]
    fn oom(&mut self, err: our::AllocErr) -> ! {
        their::Alloc::oom(self, err.into())
    }

    #[inline]
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: our::Layout) {
        their::Alloc::dealloc(self, ptr, layout.into())
    }

    #[inline]
    fn usable_size(&self, layout: &our::Layout) -> (usize, usize) {
        let layout = unsafe {
            their::Layout::from_size_align_unchecked(layout.size(), layout.align())
        };
        their::Alloc::usable_size(self, &layout)
    }

    #[inline]
    unsafe fn realloc(&mut self,
                      ptr: *mut u8,
                      layout: our::Layout,
                      new_layout: our::Layout)
                      -> Result<*mut u8, our::AllocErr>
    {
        Ok(their::Alloc::realloc(self, ptr, layout.into(), new_layout.into())?)
    }

    #[inline]
    unsafe fn alloc_zeroed(&mut self, layout: our::Layout) -> Result<*mut u8, our::AllocErr> {
        Ok(their::Alloc::alloc_zeroed(self, layout.into())?)
    }

    #[inline]
    unsafe fn alloc_excess(&mut self, layout: our::Layout) -> Result<our::Excess, our::AllocErr> {
        Ok(their::Alloc::alloc_excess(self, layout.into())?.into())
    }

    #[inline]
    unsafe fn realloc_excess(&mut self,
                             ptr: *mut u8,
                             layout: our::Layout,
                             new_layout: our::Layout) -> Result<our::Excess, our::AllocErr> {
        Ok(their::Alloc::realloc_excess(self, ptr, layout.into(), new_layout.into())?.into())
    }

    #[inline]
    unsafe fn grow_in_place(&mut self,
                            ptr: *mut u8,
                            layout: our::Layout,
                            new_layout: our::Layout)
                            -> Result<(), our::CannotReallocInPlace>
    {
        Ok(their::Alloc::grow_in_place(self, ptr, layout.into(), new_layout.into())?)
    }

    #[inline]
    unsafe fn shrink_in_place(&mut self,
                              ptr: *mut u8,
                              layout: our::Layout,
                              new_layout: our::Layout) -> Result<(), our::CannotReallocInPlace>
    {
        Ok(their::Alloc::shrink_in_place(self, ptr, layout.into(), new_layout.into())?)
    }
}

impl From<our::Layout> for their::Layout {
    #[inline]
    fn from(l: our::Layout) -> Self {
        unsafe { their::Layout::from_size_align_unchecked(l.size(), l.align()) }
    }
}

impl From<their::Layout> for our::Layout {
    #[inline]
    fn from(l: their::Layout) -> Self {
        unsafe { our::Layout::from_size_align_unchecked(l.size(), l.align()) }
    }
}

impl From<their::AllocErr> for our::AllocErr {
    #[inline]
    fn from(e: their::AllocErr) -> Self {
        match e {
            their::AllocErr::Exhausted { request: r } => our::AllocErr::Exhausted { request: r.into() },
            their::AllocErr::Unsupported { details: d } => our::AllocErr::Unsupported { details: d },
        }
    }
}

impl From<our::AllocErr> for their::AllocErr {
    #[inline]
    fn from(e: our::AllocErr) -> Self {
        match e {
            our::AllocErr::Exhausted { request: r } => their::AllocErr::Exhausted { request: r.into() },
            our::AllocErr::Unsupported { details: d } => their::AllocErr::Unsupported { details: d },
        }
    }
}

impl From<their::Excess> for our::Excess {
    #[inline]
    fn from(e: their::Excess) -> Self {
        our::Excess(e.0, e.1)
    }
}

impl From<their::CannotReallocInPlace> for our::CannotReallocInPlace {
    #[inline]
    fn from(_: their::CannotReallocInPlace) -> Self {
        our::CannotReallocInPlace
    }
}
