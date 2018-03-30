/// A dummy allocator for doc tests.

mod dummy {

    use allocator_api::{Alloc, AllocErr, Layout};

    pub struct MyHeap;

    static mut HEAP_BUF: [u8; 4096] = [0; 4096];
    static mut HEAP_CURSOR: usize = 0;

    unsafe impl<'a> Alloc for MyHeap {
        unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
            let ptr = HEAP_BUF.as_ptr() as usize;
            let mut start = HEAP_CURSOR;
            let modulo = (ptr + start) & (layout.align() - 1);
            if modulo != 0 {
                start += layout.align() - modulo;
            }
            assert_eq!((ptr + start) & (layout.align() - 1), 0);
            let end = start + layout.size();
            let buf = HEAP_BUF.get_mut(start..end);
            HEAP_CURSOR = end;
            buf.map(|b| b.as_mut_ptr())
                .ok_or_else(|| AllocErr::Exhausted { request: layout })
        }
        unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {}
    }

}

use dummy::MyHeap;
