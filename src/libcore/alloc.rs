//! Memory allocation APIs

use core::cmp;
use core::fmt;
use core::mem;
use core::ptr::{self, NonNull};
use core::usize;

/// Represents the combination of a starting address and
/// a total capacity of the returned block.
#[derive(Debug)]
pub struct Excess(pub NonNull<u8>, pub usize);

pub use core::alloc::{Layout, LayoutErr};

pub(crate) trait LayoutExt: Sized {
    fn padding_needed_for(&self, align: usize) -> usize;

    fn repeat(&self, n: usize) -> Result<(Self, usize), LayoutErr>;

    fn array<T>(n: usize) -> Result<Self, LayoutErr>;
}

fn layouterr() -> LayoutErr {
    // We can't create a LayoutErr manually in LayoutExt, but
    // Layout::from_size_align will return us one if it's given
    // wrong arguments, which an alignment of 0 is.
    Layout::from_size_align(0, 0).err().unwrap()
}

/// Implements the unstable parts of the core::alloc::Layout type that
/// this crate uses.
impl LayoutExt for Layout {
    /// Returns the amount of padding we must insert after `self`
    /// to ensure that the following address will satisfy `align`
    /// (measured in bytes).
    ///
    /// e.g., if `self.size()` is 9, then `self.padding_needed_for(4)`
    /// returns 3, because that is the minimum number of bytes of
    /// padding required to get a 4-aligned address (assuming that the
    /// corresponding memory block starts at a 4-aligned address).
    ///
    /// The return value of this function has no meaning if `align` is
    /// not a power-of-two.
    ///
    /// Note that the utility of the returned value requires `align`
    /// to be less than or equal to the alignment of the starting
    /// address for the whole allocated block of memory. One way to
    /// satisfy this constraint is to ensure `align <= self.align()`.
    #[inline]
    fn padding_needed_for(&self, align: usize) -> usize {
        let len = self.size();

        // Rounded up value is:
        //   len_rounded_up = (len + align - 1) & !(align - 1);
        // and then we return the padding difference: `len_rounded_up - len`.
        //
        // We use modular arithmetic throughout:
        //
        // 1. align is guaranteed to be > 0, so align - 1 is always
        //    valid.
        //
        // 2. `len + align - 1` can overflow by at most `align - 1`,
        //    so the &-mask with `!(align - 1)` will ensure that in the
        //    case of overflow, `len_rounded_up` will itself be 0.
        //    Thus the returned padding, when added to `len`, yields 0,
        //    which trivially satisfies the alignment `align`.
        //
        // (Of course, attempts to allocate blocks of memory whose
        // size and padding overflow in the above manner should cause
        // the allocator to yield an error anyway.)

        let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        len_rounded_up.wrapping_sub(len)
    }

    /// Creates a layout describing the record for `n` instances of
    /// `self`, with a suitable amount of padding between each to
    /// ensure that each instance is given its requested size and
    /// alignment. On success, returns `(k, offs)` where `k` is the
    /// layout of the array and `offs` is the distance between the start
    /// of each element in the array.
    ///
    /// On arithmetic overflow, returns `LayoutErr`.
    #[inline]
    fn repeat(&self, n: usize) -> Result<(Self, usize), LayoutErr> {
        // This cannot overflow. Quoting from the invariant of Layout:
        // > `size`, when rounded up to the nearest multiple of `align`,
        // > must not overflow (i.e., the rounded value must be less than
        // > `usize::MAX`)
        let padded_size = self.size() + LayoutExt::padding_needed_for(self, self.align());
        let alloc_size = padded_size.checked_mul(n).ok_or_else(layouterr)?;

        unsafe {
            // self.align is already known to be valid and alloc_size has been
            // padded already.
            Ok((
                Layout::from_size_align_unchecked(alloc_size, self.align()),
                padded_size,
            ))
        }
    }
    /// Creates a layout describing the record for a `[T; n]`.
    ///
    /// On arithmetic overflow, returns `LayoutErr`.
    #[inline]
    fn array<T>(n: usize) -> Result<Self, LayoutErr> {
        LayoutExt::repeat(&Layout::new::<T>(), n).map(|(k, offs)| {
            debug_assert!(offs == mem::size_of::<T>());
            k
        })
    }
}

/// The `AllocErr` error indicates an allocation failure
/// that may be due to resource exhaustion or to
/// something wrong when combining the given input arguments with this
/// allocator.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AllocErr;

// (we need this for downstream impl of trait Error)
impl fmt::Display for AllocErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("memory allocation failed")
    }
}

/// The `CannotReallocInPlace` error is used when [`grow_in_place`] or
/// [`shrink_in_place`] were unable to reuse the given memory block for
/// a requested layout.
///
/// [`grow_in_place`]: ./trait.AllocRef.html#method.grow_in_place
/// [`shrink_in_place`]: ./trait.AllocRef.html#method.shrink_in_place
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CannotReallocInPlace;

impl CannotReallocInPlace {
    pub fn description(&self) -> &str {
        "cannot reallocate allocator's memory in place"
    }
}

// (we need this for downstream impl of trait Error)
impl fmt::Display for CannotReallocInPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// An implementation of `AllocRef` can allocate, reallocate, and
/// deallocate arbitrary blocks of data described via `Layout`.
///
/// `AllocRef` is designed to be implemented on ZSTs, references, or
/// smart pointers because having an allocator like `MyAlloc([u8; N])`
/// cannot be moved, without updating the pointers to the allocated
/// memory.
///
/// Some of the methods require that a memory block be *currently
/// allocated* via an allocator. This means that:
///
/// * the starting address for that memory block was previously
///   returned by a previous call to an allocation method (`alloc`,
///   `alloc_zeroed`) or reallocation method (`realloc`), and
///
/// * the memory block has not been subsequently deallocated, where
///   blocks are deallocated either by being passed to a deallocation
///   method (`dealloc`) or by being passed to a reallocation method
///  (see above) that returns `Ok`.
///
/// Unlike [`GlobalAlloc`], zero-sized allocations are allowed in
/// `AllocRef`. If an underlying allocator does not support this (like
/// jemalloc) or return a null pointer (such as `libc::malloc`), this case
/// must be caught. In this case [`Layout::dangling()`] can be used to
/// create a dangling, but aligned `NonNull<u8>`.
///
/// Some of the methods require that a layout *fit* a memory block.
/// What it means for a layout to "fit" a memory block means (or
/// equivalently, for a memory block to "fit" a layout) is that the
/// following two conditions must hold:
///
/// 1. The block's starting address must be aligned to `layout.align()`.
///
/// 2. The block's size must fall in the range `[use_min, use_max]`, where:
///
///    * `use_min` is `layout.size()`, and
///
///    * `use_max` is the capacity that was returned.
///
/// Note that:
///
///  * the size of the layout most recently used to allocate the block
///    is guaranteed to be in the range `[use_min, use_max]`, and
///
///  * a lower-bound on `use_max` can be safely approximated by a call to
///    `usable_size`.
///
///  * if a layout `k` fits a memory block (denoted by `ptr`)
///    currently allocated via an allocator `a`, then it is legal to
///    use that layout to deallocate it, i.e., `a.dealloc(ptr, k);`.
///
///  * if an allocator does not support overallocating, it is fine to
///    simply return `layout.size()` as the allocated size.
///
/// [`GlobalAlloc`]: self::GlobalAlloc
/// [`Layout::dangling()`]: self::Layout::dangling
///
/// # Safety
///
/// The `AllocRef` trait is an `unsafe` trait for a number of reasons, and
/// implementors must ensure that they adhere to these contracts:
///
/// * Pointers returned from allocation functions must point to valid memory and
///   retain their validity until at least one instance of `AllocRef` is dropped
///   itself.
///
/// * Cloning or moving the allocator must not invalidate pointers returned
///   from this allocator. Cloning must return a reference to the same allocator.
///
/// * `Layout` queries and calculations in general must be correct. Callers of
///   this trait are allowed to rely on the contracts defined on each method,
///   and implementors must ensure such contracts remain true.
///
/// Note that this list may get tweaked over time as clarifications are made in
/// the future.
pub unsafe trait AllocRef {
    /// On success, returns a pointer meeting the size and alignment
    /// guarantees of `layout` and the actual size of the allocated block,
    /// which must be greater than or equal to `layout.size()`.
    ///
    /// If this method returns an `Ok(addr)`, then the `addr` returned
    /// will be non-null address pointing to a block of storage
    /// suitable for holding an instance of `layout`.
    ///
    /// The returned block of storage may or may not have its contents
    /// initialized. (Extension subtraits might restrict this
    /// behavior, e.g., to ensure initialization to particular sets of
    /// bit patterns.)
    ///
    /// # Errors
    ///
    /// Returning `Err` indicates that either memory is exhausted or
    /// `layout` does not meet allocator's size or alignment
    /// constraints.
    ///
    /// Implementations are encouraged to return `Err` on memory
    /// exhaustion rather than panicking or aborting, but this is not
    /// a strict requirement. (Specifically: it is *legal* to
    /// implement this trait atop an underlying native allocation
    /// library that aborts on memory exhaustion.)
    ///
    /// Clients wishing to abort computation in response to an
    /// allocation error are encouraged to call the [`handle_alloc_error`] function,
    /// rather than directly invoking `panic!` or similar.
    ///
    /// [`handle_alloc_error`]: ../../alloc/alloc/fn.handle_alloc_error.html
    fn alloc(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), AllocErr>;

    /// Deallocate the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure all of the following:
    ///
    /// * `ptr` must denote a block of memory currently allocated via
    ///   this allocator,
    ///
    /// * `layout` must *fit* that block of memory,
    ///
    /// * In addition to fitting the block of memory `layout`, the
    ///   alignment of the `layout` must match the alignment used
    ///   to allocate that block of memory.
    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout);

    /// Behaves like `alloc`, but also ensures that the contents
    /// are set to zero before being returned.
    ///
    /// # Errors
    ///
    /// Returning `Err` indicates that either memory is exhausted or
    /// `layout` does not meet allocator's size or alignment
    /// constraints, just as in `alloc`.
    ///
    /// Clients wishing to abort computation in response to an
    /// allocation error are encouraged to call the [`handle_alloc_error`] function,
    /// rather than directly invoking `panic!` or similar.
    ///
    /// [`handle_alloc_error`]: ../../alloc/alloc/fn.handle_alloc_error.html
    fn alloc_zeroed(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), AllocErr> {
        let size = layout.size();
        let result = self.alloc(layout);
        if let Ok((p, _)) = result {
            unsafe { ptr::write_bytes(p.as_ptr(), 0, size) }
        }
        result
    }

    // == METHODS FOR MEMORY REUSE ==
    // realloc, realloc_zeroed, grow_in_place, grow_in_place_zeroed, shrink_in_place

    /// Returns a pointer suitable for holding data described by
    /// a new layout with `layout`’s alignment and a size given
    /// by `new_size` and the actual size of the allocated block.
    /// The latter is greater than or equal to `layout.size()`.
    /// To accomplish this, the allocator may extend or shrink
    /// the allocation referenced by `ptr` to fit the new layout.
    ///
    /// If this returns `Ok`, then ownership of the memory block
    /// referenced by `ptr` has been transferred to this
    /// allocator. The memory may or may not have been freed, and
    /// should be considered unusable (unless of course it was
    /// transferred back to the caller again via the return value of
    /// this method).
    ///
    /// If this method returns `Err`, then ownership of the memory
    /// block has not been transferred to this allocator, and the
    /// contents of the memory block are unaltered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure all of the following:
    ///
    /// * `ptr` must be currently allocated via this allocator,
    ///
    /// * `layout` must *fit* the `ptr` (see above). (The `new_size`
    ///   argument need not fit it.)
    ///
    /// * `new_size`, when rounded up to the nearest multiple of `layout.align()`,
    ///   must not overflow (i.e., the rounded value must be less than `usize::MAX`).
    ///
    /// (Extension subtraits might provide more specific bounds on
    /// behavior, e.g., guarantee a sentinel address or a null pointer
    /// in response to a zero-size allocation request.)
    ///
    /// # Errors
    ///
    /// Returns `Err` only if the new layout
    /// does not meet the allocator's size
    /// and alignment constraints of the allocator, or if reallocation
    /// otherwise fails.
    ///
    /// Implementations are encouraged to return `Err` on memory
    /// exhaustion rather than panicking or aborting, but this is not
    /// a strict requirement. (Specifically: it is *legal* to
    /// implement this trait atop an underlying native allocation
    /// library that aborts on memory exhaustion.)
    ///
    /// Clients wishing to abort computation in response to a
    /// reallocation error are encouraged to call the [`handle_alloc_error`] function,
    /// rather than directly invoking `panic!` or similar.
    ///
    /// [`handle_alloc_error`]: ../../alloc/alloc/fn.handle_alloc_error.html
    unsafe fn realloc(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<(NonNull<u8>, usize), AllocErr> {
        let old_size = layout.size();

        if new_size > old_size {
            if let Ok(size) = self.grow_in_place(ptr, layout, new_size) {
                return Ok((ptr, size));
            }
        } else if new_size < old_size {
            if let Ok(size) = self.shrink_in_place(ptr, layout, new_size) {
                return Ok((ptr, size));
            }
        } else {
            return Ok((ptr, new_size));
        }

        // otherwise, fall back on alloc + copy + dealloc.
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let result = self.alloc(new_layout);
        if let Ok((new_ptr, _)) = result {
            ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr(), cmp::min(old_size, new_size));
            self.dealloc(ptr, layout);
        }
        result
    }

    /// Behaves like `realloc`, but also ensures that the new contents
    /// are set to zero before being returned.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the same reasons that `realloc` is.
    ///
    /// # Errors
    ///
    /// Returns `Err` only if the new layout
    /// does not meet the allocator's size
    /// and alignment constraints of the allocator, or if reallocation
    /// otherwise fails.
    ///
    /// Implementations are encouraged to return `Err` on memory
    /// exhaustion rather than panicking or aborting, but this is not
    /// a strict requirement. (Specifically: it is *legal* to
    /// implement this trait atop an underlying native allocation
    /// library that aborts on memory exhaustion.)
    ///
    /// Clients wishing to abort computation in response to a
    /// reallocation error are encouraged to call the [`handle_alloc_error`] function,
    /// rather than directly invoking `panic!` or similar.
    ///
    /// [`handle_alloc_error`]: ../../alloc/alloc/fn.handle_alloc_error.html
    unsafe fn realloc_zeroed(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<(NonNull<u8>, usize), AllocErr> {
        let old_size = layout.size();

        if new_size > old_size {
            if let Ok(size) = self.grow_in_place_zeroed(ptr, layout, new_size) {
                return Ok((ptr, size));
            }
        } else if new_size < old_size {
            if let Ok(size) = self.shrink_in_place(ptr, layout, new_size) {
                return Ok((ptr, size));
            }
        } else {
            return Ok((ptr, new_size));
        }

        // otherwise, fall back on alloc + copy + dealloc.
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let result = self.alloc_zeroed(new_layout);
        if let Ok((new_ptr, _)) = result {
            ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr(), cmp::min(old_size, new_size));
            self.dealloc(ptr, layout);
        }
        result
    }

    /// Attempts to extend the allocation referenced by `ptr` to fit `new_size`.
    ///
    /// If this returns `Ok`, then the allocator has asserted that the
    /// memory block referenced by `ptr` now fits `new_size`, and thus can
    /// be used to carry data of a layout of that size and same alignment as
    /// `layout`. The returned value is the new size of the allocated block.
    /// (The allocator is allowed to expend effort to accomplish this, such
    /// as extending the memory block to include successor blocks, or virtual
    /// memory tricks.)
    ///
    /// Regardless of what this method returns, ownership of the
    /// memory block referenced by `ptr` has not been transferred, and
    /// the contents of the memory block are unaltered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure all of the following:
    ///
    /// * `ptr` must be currently allocated via this allocator,
    ///
    /// * `layout` must *fit* the `ptr` (see above); note the
    ///   `new_size` argument need not fit it,
    ///
    /// * `new_size` must not be less than `layout.size()`,
    ///
    /// # Errors
    ///
    /// Returns `Err(CannotReallocInPlace)` when the allocator is
    /// unable to assert that the memory block referenced by `ptr`
    /// could fit `layout`.
    ///
    /// Note that one cannot pass `CannotReallocInPlace` to the `handle_alloc_error`
    /// function; clients are expected either to be able to recover from
    /// `grow_in_place` failures without aborting, or to fall back on
    /// another reallocation method before resorting to an abort.
    #[inline]
    unsafe fn grow_in_place(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<usize, CannotReallocInPlace> {
        let _ = ptr;
        let _ = layout;
        let _ = new_size;
        Err(CannotReallocInPlace)
    }

    /// Behaves like `grow_in_place`, but also ensures that the new
    /// contents are set to zero before being returned.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the same reasons that `grow_in_place` is.
    ///
    /// # Errors
    ///
    /// Returns `Err(CannotReallocInPlace)` when the allocator is
    /// unable to assert that the memory block referenced by `ptr`
    /// could fit `layout`.
    ///
    /// Note that one cannot pass `CannotReallocInPlace` to the `handle_alloc_error`
    /// function; clients are expected either to be able to recover from
    /// `grow_in_place` failures without aborting, or to fall back on
    /// another reallocation method before resorting to an abort.
    unsafe fn grow_in_place_zeroed(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<usize, CannotReallocInPlace> {
        let size = self.grow_in_place(ptr, layout, new_size)?;
        ptr.as_ptr().add(layout.size()).write_bytes(0, new_size - layout.size());
        Ok(size)
    }

    /// Attempts to shrink the allocation referenced by `ptr` to fit `new_size`.
    ///
    /// If this returns `Ok`, then the allocator has asserted that the
    /// memory block referenced by `ptr` now fits `new_size`, and
    /// thus can only be used to carry data of that smaller
    /// layout. The returned value is the new size the allocated block.
    /// (The allocator is allowed to take advantage of this,
    /// carving off portions of the block for reuse elsewhere.) The
    /// truncated contents of the block within the smaller layout are
    /// unaltered, and ownership of block has not been transferred.
    ///
    /// If this returns `Err`, then the memory block is considered to
    /// still represent the original (larger) `layout`. None of the
    /// block has been carved off for reuse elsewhere, ownership of
    /// the memory block has not been transferred, and the contents of
    /// the memory block are unaltered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure all of the following:
    ///
    /// * `ptr` must be currently allocated via this allocator,
    ///
    /// * `layout` must *fit* the `ptr` (see above); note the
    ///   `new_size` argument need not fit it,
    ///
    /// * `new_size` must not be greater than `layout.size()`,
    ///
    /// # Errors
    ///
    /// Returns `Err(CannotReallocInPlace)` when the allocator is
    /// unable to assert that the memory block referenced by `ptr`
    /// could fit `layout`.
    ///
    /// Note that one cannot pass `CannotReallocInPlace` to the `handle_alloc_error`
    /// function; clients are expected either to be able to recover from
    /// `shrink_in_place` failures without aborting, or to fall back
    /// on another reallocation method before resorting to an abort.
    #[inline]
    unsafe fn shrink_in_place(
        &mut self,
        ptr: NonNull<u8>,
        layout: Layout,
        new_size: usize,
    ) -> Result<usize, CannotReallocInPlace> {
        let _ = ptr;
        let _ = layout;
        let _ = new_size;
        Err(CannotReallocInPlace)
    }
}
