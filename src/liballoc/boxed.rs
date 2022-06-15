//! A pointer type for heap allocation.
//!
//! `Box<T, A>` is similar to
//! [`std::boxed::Box<T>`](https://doc.rust-lang.org/nightly/std/boxed/struct.Box.html),
//! but pointers are associated with a specific allocator, allowing boxed pointers
//! in different heaps.

use core::borrow;
use core::cmp::Ordering;
use core::convert::From;
use core::fmt;
use core::future::Future;
use core::hash::{Hash, Hasher};
use core::iter::{Iterator, FusedIterator};
use core::marker::Unpin;
use core::mem;
use core::pin::Pin;
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull};
use core::task::{Context, Poll};

use crate::alloc::{Alloc, Layout, handle_alloc_error};
#[cfg(feature = "std")]
use crate::alloc::Global;
use crate::raw_vec::RawVec;
use crate::Unique;

/// A pointer type for heap allocation.
global_alloc! {
    pub struct Box<T: ?Sized, A: Alloc>(Unique<T>, pub(crate) A);
}

impl<T, A: Alloc> Box<T, A> {
    /// Allocates memory in the given allocator and then places `x` into it.
    ///
    /// This doesn't actually allocate if `T` is zero-sized.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::{Box, Global};
    /// let five = Box::new_in(5, Global);
    /// # }
    /// ```
    #[inline(always)]
    pub fn new_in(x: T, a: A) -> Box<T, A> {
        let mut a = a;
        let layout = Layout::for_value(&x);
        let size = layout.size();
        let ptr = if size == 0 {
            NonNull::dangling()
        } else {
            unsafe {
                let ptr = a.alloc(layout).unwrap_or_else(|_| { handle_alloc_error(layout) });
                ptr.cast()
            }
        };
        unsafe {
            ptr::write(ptr.as_ptr() as *mut T, x);
        }
        Box(ptr.into(), a)
    }

    /// Constructs a new `Pin<Box<T>>`. If `T` does not implement `Unpin`, then
    /// `x` will be pinned in memory and unable to be moved.
    #[inline(always)]
    pub fn pin_in(x: T, a: A) -> Pin<Box<T, A>> {
        Box::new_in(x, a).into()
    }

}

#[cfg(feature = "std")]
impl<T> Box<T> {
    /// Allocates memory on the heap and then places `x` into it.
    ///
    /// This doesn't actually allocate if `T` is zero-sized.
    ///
    /// # Examples
    ///
    /// ```
    /// use allocator_api::Box;
    /// let five = Box::new(5);
    /// ```
    #[inline(always)]
    pub fn new(x: T) -> Box<T> {
        Box::new_in(x, Global)
    }

    /// Constructs a new `Pin<Box<T>>`. If `T` does not implement `Unpin`, then
    /// `x` will be pinned in memory and unable to be moved.
    #[inline(always)]
    pub fn pin(x: T) -> Pin<Box<T>> {
        Box::new(x).into()
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized> Box<T> {
    /// Constructs a box from a raw pointer.
    ///
    /// After calling this function, the raw pointer is owned by the
    /// resulting `Box`. Specifically, the `Box` destructor will call
    /// the destructor of `T` and free the allocated memory. For this
    /// to be safe, the memory must have been allocated in accordance
    /// with the [memory layout] used by `Box` .
    ///
    /// # Safety
    ///
    /// This function is unsafe because improper use may lead to
    /// memory problems. For example, a double-free may occur if the
    /// function is called twice on the same raw pointer.
    ///
    /// # Examples
    /// Recreate a `Box` which was previously converted to a raw pointer
    /// using [`Box::into_raw`]:
    /// ```
    /// use allocator_api::Box;
    /// let x = Box::new(5);
    /// let ptr = Box::into_raw(x);
    /// let x = unsafe { Box::from_raw(ptr) };
    /// ```
    /// Manually create a `Box` from scratch by using the global allocator:
    /// ```
    /// use allocator_api::Box;
    /// use std::alloc::{alloc, Layout};
    ///
    /// unsafe {
    ///     let ptr = alloc(Layout::new::<i32>()) as *mut i32;
    ///     *ptr = 5;
    ///     let x = Box::from_raw(ptr);
    /// }
    /// ```
    ///
    /// [memory layout]: https://doc.rust-lang.org/std/boxed/index.html#memory-layout
    /// [`Layout`]: ../alloc/struct.Layout.html
    /// [`Box::into_raw`]: struct.Box.html#method.into_raw
    #[inline]
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Box::from_raw_in(raw, Global)
    }
}

impl<T: ?Sized, A: Alloc> Box<T, A> {
    /// Constructs a box from a raw pointer in the given allocator.
    ///
    /// This is similar to the [`Box::from_raw`] function, but assumes
    /// the pointer was allocated with the given allocator.
    ///
    /// This function is unsafe because improper use may lead to
    /// memory problems. For example, specifying the wrong allocator
    /// may corrupt the allocator state.
    ///
    /// [`Box::into_raw`]: struct.Box.html#method.into_raw
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::{Box, Global};
    /// let x = Box::new_in(5, Global);
    /// let ptr = Box::into_raw(x);
    /// let x = unsafe { Box::from_raw_in(ptr, Global) };
    /// # }
    /// ```
    #[inline]
    pub unsafe fn from_raw_in(raw: *mut T, a: A) -> Self {
        Box(Unique::new_unchecked(raw), a)
    }

    /// Consumes the `Box`, returning a wrapped raw pointer.
    ///
    /// The pointer will be properly aligned and non-null.
    ///
    /// After calling this function, the caller is responsible for the
    /// memory previously managed by the `Box`. In particular, the
    /// caller should properly destroy `T` and release the memory, taking
    /// into account the [memory layout] used by `Box`. The easiest way to
    /// do this is to convert the raw pointer back into a `Box` with the
    /// [`Box::from_raw`] function, allowing the `Box` destructor to perform
    /// the cleanup.
    ///
    /// Note: this is an associated function, which means that you have
    /// to call it as `Box::into_raw(b)` instead of `b.into_raw()`. This
    /// is so that there is no conflict with a method on the inner type.
    ///
    /// # Examples
    /// Converting the raw pointer back into a `Box` with [`Box::from_raw`]
    /// for automatic cleanup:
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// let x = Box::new(String::from("Hello"));
    /// let ptr = Box::into_raw(x);
    /// let x = unsafe { Box::from_raw(ptr) };
    /// # }
    /// ```
    /// Manual cleanup by explicitly running the destructor and deallocating
    /// the memory:
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use std::alloc::{dealloc, Layout};
    /// use std::ptr;
    /// use allocator_api::Box;
    ///
    /// let x = Box::new(String::from("Hello"));
    /// let p = Box::into_raw(x);
    /// unsafe {
    ///     ptr::drop_in_place(p);
    ///     dealloc(p as *mut u8, Layout::new::<String>());
    /// }
    /// # }
    /// ```
    ///
    /// [memory layout]: https://doc.rust-lang.org/std/boxed/index.html#memory-layout
    /// [`Box::from_raw`]: struct.Box.html#method.from_raw
    #[inline]
    pub fn into_raw(b: Box<T, A>) -> *mut T {
        Box::into_raw_non_null(b).as_ptr()
    }

    /// Consumes the `Box`, returning the wrapped pointer as `NonNull<T>`.
    ///
    /// After calling this function, the caller is responsible for the
    /// memory previously managed by the `Box`. In particular, the
    /// caller should properly destroy `T` and release the memory. The
    /// easiest way to do so is to convert the `NonNull<T>` pointer
    /// into a raw pointer and back into a `Box` with the [`Box::from_raw`]
    /// function.
    ///
    /// Note: this is an associated function, which means that you have
    /// to call it as `Box::into_raw_non_null(b)`
    /// instead of `b.into_raw_non_null()`. This
    /// is so that there is no conflict with a method on the inner type.
    ///
    /// [`Box::from_raw`]: struct.Box.html#method.from_raw
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    ///
    /// fn main() {
    ///     let x = Box::new(5);
    ///     let ptr = Box::into_raw_non_null(x);
    ///
    ///     // Clean up the memory by converting the NonNull pointer back
    ///     // into a Box and letting the Box be dropped.
    ///     let x = unsafe { Box::from_raw(ptr.as_ptr()) };
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn into_raw_non_null(b: Box<T, A>) -> NonNull<T> {
        Box::into_unique(b).into()
    }

    pub(crate) fn into_unique(b: Box<T, A>) -> Unique<T> {
        let ptr = b.0;
        mem::forget(b);
        ptr
    }

    /// Consumes and leaks the `Box`, returning a mutable reference,
    /// `&'a mut T`. Note that the type `T` must outlive the chosen lifetime
    /// `'a`. If the type has only static references, or none at all, then this
    /// may be chosen to be `'static`.
    ///
    /// This function is mainly useful for data that lives for the remainder of
    /// the program's life. Dropping the returned reference will cause a memory
    /// leak. If this is not acceptable, the reference should first be wrapped
    /// with the [`Box::from_raw`] function producing a `Box`. This `Box` can
    /// then be dropped which will properly destroy `T` and release the
    /// allocated memory.
    ///
    /// Note: this is an associated function, which means that you have
    /// to call it as `Box::leak(b)` instead of `b.leak()`. This
    /// is so that there is no conflict with a method on the inner type.
    ///
    /// [`Box::from_raw`]: struct.Box.html#method.from_raw
    ///
    /// # Examples
    ///
    /// Simple usage:
    ///
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// fn main() {
    ///     let x = Box::new(41);
    ///     let static_ref: &'static mut usize = Box::leak(x);
    ///     *static_ref += 1;
    ///     assert_eq!(*static_ref, 42);
    /// }
    /// # }
    /// ```
    ///
    /// Unsized data:
    ///
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// # use std::ptr;
    /// use allocator_api::{Box, RawVec};
    /// struct MyVec<T> {
    ///     buf: RawVec<T>,
    ///     len: usize,
    /// }
    ///
    /// impl<T> MyVec<T> {
    ///     pub fn push(&mut self, elem: T) {
    ///         if self.len == self.buf.cap() { self.buf.double(); }
    ///         // double would have aborted or panicked if the len exceeded
    ///         // `isize::MAX` so this is safe to do unchecked now.
    ///         unsafe {
    ///             ptr::write(self.buf.ptr().offset(self.len as isize), elem);
    ///         }
    ///         self.len += 1;
    ///     }
    /// }
    /// fn main() {
    ///     //let x = vec![1, 2, 3].into_boxed_slice();
    ///     let mut v = MyVec { buf: RawVec::new(), len: 0 };
    ///     v.push(1);
    ///     v.push(2);
    ///     v.push(3);
    ///     v.buf.shrink_to_fit(v.len);
    ///     let x = unsafe { v.buf.into_box() };
    ///     let static_ref = Box::leak(x);
    ///     static_ref[0] = 4;
    ///     assert_eq!(*static_ref, [4, 2, 3]);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn leak<'a>(b: Box<T, A>) -> &'a mut T
    where
        T: 'a // Technically not needed, but kept to be explicit.
    {
        unsafe { &mut *Box::into_raw(b) }
    }

    /// Converts a `Box<T, A>` into a `Pin<Box<T, A>>`
    ///
    /// This conversion does not allocate and happens in place.
    ///
    /// This is also available via [`From`].
    pub fn into_pin(boxed: Box<T, A>) -> Pin<Box<T, A>> {
        // It's not possible to move or replace the insides of a `Pin<Box<T>>`
        // when `T: !Unpin`,  so it's safe to pin it directly without any
        // additional requirements.
        unsafe { Pin::new_unchecked(boxed) }
    }
}

impl<T: ?Sized, A: Alloc> Drop for Box<T, A> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::for_value(self.0.as_ref());
            ptr::drop_in_place(self.0.as_ptr());
            if layout.size() != 0 {
                self.1.dealloc(NonNull::from(self.0).cast(), layout);
            }
        }
    }
}

impl<T: Default, A: Alloc + Default> Default for Box<T, A> {
    /// Creates a `Box<T>`, with the `Default` value for T.
    fn default() -> Box<T, A> {
        Box::new_in(Default::default(), Default::default())
    }
}

impl<T, A: Alloc + Default> Default for Box<[T], A> {
    fn default() -> Box<[T], A> {
        let a = A::default();
        let b = Box::<[T; 0], A>::new_in([], a);
        let raw = b.0.as_ptr();
        let a = unsafe { ptr::read(&b.1) };
        mem::forget(b);
        unsafe { Box::from_raw_in(raw, a) }
    }
}

/// Converts a boxed slice of bytes to a boxed string slice without checking
/// that the string contains valid UTF-8.
#[inline]
pub unsafe fn from_boxed_utf8_unchecked<A: Alloc>(v: Box<[u8], A>) -> Box<str, A> {
    let a = ptr::read(&v.1);
    Box::from_raw_in(Box::into_raw(v) as *mut str, a)
}

impl<A: Alloc + Default> Default for Box<str, A> {
    fn default() -> Box<str, A> {
        unsafe { from_boxed_utf8_unchecked(Default::default()) }
    }
}

impl<T: Clone, A: Alloc + Clone> Clone for Box<T, A> {
    /// Returns a new box with a `clone()` of this box's contents.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// let x = Box::new(5);
    /// let y = x.clone();
    /// # }
    /// ```
    #[inline]
    fn clone(&self) -> Box<T, A> {
        Box::new_in((**self).clone(), self.1.clone())
    }
    /// Copies `source`'s contents into `self` without creating a new allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// let x = Box::new(5);
    /// let mut y = Box::new(10);
    ///
    /// y.clone_from(&x);
    ///
    /// assert_eq!(*y, 5);
    /// # }
    /// ```
    #[inline]
    fn clone_from(&mut self, source: &Box<T, A>) {
        (**self).clone_from(&(**source));
    }
}

impl<A: Alloc + Clone> Clone for Box<str, A> {
    fn clone(&self) -> Self {
        let len = self.len();
        let buf = RawVec::with_capacity_in(len, self.1.clone());
        unsafe {
            ptr::copy_nonoverlapping(self.as_ptr(), buf.ptr(), len);
            from_boxed_utf8_unchecked(buf.into_box())
        }
    }
}

impl<T: ?Sized + PartialEq, A: Alloc> PartialEq for Box<T, A> {
    #[inline]
    fn eq(&self, other: &Box<T, A>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
    #[inline]
    fn ne(&self, other: &Box<T, A>) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: ?Sized + PartialOrd, A: Alloc> PartialOrd for Box<T, A> {
    #[inline]
    fn partial_cmp(&self, other: &Box<T, A>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
    #[inline]
    fn lt(&self, other: &Box<T, A>) -> bool {
        PartialOrd::lt(&**self, &**other)
    }
    #[inline]
    fn le(&self, other: &Box<T, A>) -> bool {
        PartialOrd::le(&**self, &**other)
    }
    #[inline]
    fn ge(&self, other: &Box<T, A>) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
    #[inline]
    fn gt(&self, other: &Box<T, A>) -> bool {
        PartialOrd::gt(&**self, &**other)
    }
}

impl<T: ?Sized + Ord, A: Alloc> Ord for Box<T, A> {
    #[inline]
    fn cmp(&self, other: &Box<T, A>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: ?Sized + Eq, A: Alloc> Eq for Box<T, A> {}

impl<T: ?Sized + Hash, A: Alloc> Hash for Box<T, A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: ?Sized + Hasher, A: Alloc> Hasher for Box<T, A> {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i)
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i)
    }
    fn write_u128(&mut self, i: u128) {
        (**self).write_u128(i)
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i)
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i)
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i)
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i)
    }
    fn write_i64(&mut self, i: i64) {
        (**self).write_i64(i)
    }
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i)
    }
}

impl<T, A: Alloc + Default> From<T> for Box<T, A> {
    /// Converts a generic type `T` into a `Box<T, A>`
    ///
    /// The conversion allocates with the associated allocator and moves `t`
    /// from the stack into it.
    ///
    /// # Examples
    /// ```rust
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// let x = 5;
    /// let boxed = Box::new(5);
    ///
    /// assert_eq!(Box::from(x), boxed);
    /// # }
    /// ```
    fn from(t: T) -> Self {
        Box::new_in(t, Default::default())
    }
}

impl<T: ?Sized, A: Alloc> From<Box<T, A>> for Pin<Box<T, A>> {
    /// Converts a `Box<T, A>` into a `Pin<Box<T, A>>`
    ///
    /// This conversion does not allocate and happens in place.
    fn from(boxed: Box<T, A>) -> Self {
        Box::into_pin(boxed)
    }
}

impl<T: Copy, A: Alloc + Default> From<&[T]> for Box<[T], A> {
    /// Converts a `&[T]` into a `Box<[T], A>`
    ///
    /// This conversion allocates with the associated allocator
    /// and performs a copy of `slice`.
    ///
    /// # Examples
    /// ```rust
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// // create a &[u8] which will be used to create a Box<[u8]>
    /// let slice: &[u8] = &[104, 101, 108, 108, 111];
    /// let boxed_slice: Box<[u8]> = Box::from(slice);
    ///
    /// println!("{:?}", boxed_slice);
    /// # }
    /// ```
    fn from(slice: &[T]) -> Box<[T], A> {
        let len = slice.len();
        let a = Default::default();
        let buf = RawVec::with_capacity_in(len, a);
        unsafe {
            ptr::copy_nonoverlapping(slice.as_ptr(), buf.ptr(), len);
            buf.into_box()
        }
    }
}

impl<A: Alloc + Default> From<&str> for Box<str, A> {
    /// Converts a `&str` into a `Box<str, A>`
    ///
    /// This conversion allocates with the associated allocator
    /// and performs a copy of `s`.
    ///
    /// # Examples
    /// ```rust
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// let boxed: Box<str> = Box::from("hello");
    /// println!("{}", boxed);
    /// # }
    /// ```
    #[inline]
    fn from(s: &str) -> Box<str, A> {
        unsafe { from_boxed_utf8_unchecked(Box::from(s.as_bytes())) }
    }
}

impl<A: Alloc> From<Box<str, A>> for Box<[u8], A> {
    /// Converts a `Box<str, A>` into a `Box<[u8], A>`
    ///
    /// This conversion does not allocate on the heap and happens in place.
    ///
    /// # Examples
    /// ```rust
    /// # #[macro_use] extern crate allocator_api;
    /// # test_using_global! {
    /// use allocator_api::Box;
    /// // create a Box<str> which will be used to create a Box<[u8]>
    /// let boxed: Box<str> = Box::from("hello");
    /// let boxed_str: Box<[u8]> = Box::from(boxed);
    ///
    /// // create a &[u8] which will be used to create a Box<[u8]>
    /// let slice: &[u8] = &[104, 101, 108, 108, 111];
    /// let boxed_slice = Box::from(slice);
    ///
    /// assert_eq!(boxed_slice, boxed_str);
    /// # }
    /// ```
    #[inline]
    fn from(s: Box<str, A>) -> Self {
        unsafe {
            let a = ptr::read(&s.1);
            Box::from_raw_in(Box::into_raw(s) as *mut [u8], a)
        }
    }
}

impl<T: fmt::Display + ?Sized, A: Alloc> fmt::Display for Box<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug + ?Sized, A: Alloc> fmt::Debug for Box<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized, A: Alloc> fmt::Pointer for Box<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // It's not possible to extract the inner Uniq directly from the Box,
        // instead we cast it to a *const which aliases the Unique
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl<T: ?Sized, A: Alloc> Deref for Box<T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized, A: Alloc> DerefMut for Box<T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<I: Iterator + ?Sized, A: Alloc> Iterator for Box<I, A> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        (**self).nth(n)
    }
}

impl<I: DoubleEndedIterator + ?Sized, A: Alloc> DoubleEndedIterator for Box<I, A> {
    fn next_back(&mut self) -> Option<I::Item> {
        (**self).next_back()
    }
    fn nth_back(&mut self, n: usize) -> Option<I::Item> {
        (**self).nth_back(n)
    }
}

impl<I: ExactSizeIterator + ?Sized, A: Alloc> ExactSizeIterator for Box<I, A> {
    fn len(&self) -> usize {
        (**self).len()
    }
}

impl<I: FusedIterator + ?Sized, A: Alloc> FusedIterator for Box<I, A> {}

impl<T: Clone, A: Alloc + Clone> Clone for Box<[T], A> {
    fn clone(&self) -> Self {
        let mut new = BoxBuilder {
            data: RawVec::with_capacity_in(self.len(), self.1.clone()),
            len: 0,
        };

        let mut target = new.data.ptr();

        for item in self.iter() {
            unsafe {
                ptr::write(target, item.clone());
                target = target.offset(1);
            };

            new.len += 1;
        }

        return unsafe { new.into_box() };

        // Helper type for responding to panics correctly.
        struct BoxBuilder<T, A: Alloc> {
            data: RawVec<T, A>,
            len: usize,
        }

        impl<T, A: Alloc> BoxBuilder<T, A> {
            unsafe fn into_box(self) -> Box<[T], A> {
                let raw = ptr::read(&self.data);
                mem::forget(self);
                raw.into_box()
            }
        }

        impl<T, A: Alloc> Drop for BoxBuilder<T, A> {
            fn drop(&mut self) {
                let mut data = self.data.ptr();
                let max = unsafe { data.add(self.len) };

                while data != max {
                    unsafe {
                        ptr::read(data);
                        data = data.offset(1);
                    }
                }
            }
        }
    }
}

impl<T: ?Sized, A: Alloc> borrow::Borrow<T> for Box<T, A> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, A: Alloc> borrow::BorrowMut<T> for Box<T, A> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized, A: Alloc> AsRef<T> for Box<T, A> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, A: Alloc> AsMut<T> for Box<T, A> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

/* Nota bene
 *
 *  We could have chosen not to add this impl, and instead have written a
 *  function of Pin<Box<T>> to Pin<T>. Such a function would not be sound,
 *  because Box<T> implements Unpin even when T does not, as a result of
 *  this impl.
 *
 *  We chose this API instead of the alternative for a few reasons:
 *      - Logically, it is helpful to understand pinning in regard to the
 *        memory region being pointed to. For this reason none of the
 *        standard library pointer types support projecting through a pin
 *        (Box<T> is the only pointer type in std for which this would be
 *        safe.)
 *      - It is in practice very useful to have Box<T> be unconditionally
 *        Unpin because of trait objects, for which the structural auto
 *        trait functionality does not apply (e.g., Box<dyn Foo> would
 *        otherwise not be Unpin).
 *
 *  Another type with the same semantics as Box but only a conditional
 *  implementation of `Unpin` (where `T: Unpin`) would be valid/safe, and
 *  could have a method to project a Pin<T> from it.
 */
impl<T: ?Sized, A: Alloc> Unpin for Box<T, A> { }

impl<F: ?Sized + Future + Unpin, A: Alloc> Future for Box<F, A> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        F::poll(Pin::new(&mut *self), cx)
    }
}
