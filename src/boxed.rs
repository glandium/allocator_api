// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A pointer type for heap allocation.
//!
//! `Box<T, A>` is similar to
//! [`std::boxed::Box<T>`](https://doc.rust-lang.org/nightly/std/boxed/struct.Box.html),
//! but pointers are associated with a specific allocator, allowing boxed pointers
//! in different heaps.

use core::borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
#[cfg(feature = "fused")]
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull};
use allocator::{Alloc, Layout};
#[cfg(feature = "heap")]
use heap::Heap;
use raw_vec::RawVec;

macro_rules! box_ {
    ($($default:ty)*) => {
        /// A pointer type for heap allocation.
        pub struct Box<T: ?Sized, A: Alloc $(= $default)*> {
            ptr: NonNull<T>,
            marker: PhantomData<T>,
            pub(crate) a: A,
        }
    };
}

#[cfg(feature = "heap")]
box_!(Heap);

#[cfg(not(feature = "heap"))]
box_!();

impl<T, A: Alloc> Box<T, A> {
    /// Allocates memory in the given allocator and then places `x` into it.
    ///
    /// This doesn't actually allocate if `T` is zero-sized.
    #[inline(always)]
    pub fn new_in(x: T, a: A) -> Box<T, A> {
        let mut a = a;
        let layout = Layout::for_value(&x);
        let size = layout.size();
        let ptr = if size == 0 {
            NonNull::dangling()
        } else {
            unsafe {
                let ptr = a.alloc(layout).unwrap_or_else(|err| { a.oom(err) }) as *mut T;
                ptr::copy_nonoverlapping(&x, ptr, size);
                // Wishful thinking: the allocator didn't return null.
                NonNull::new_unchecked(ptr)
            }
        };
        Box {
            ptr: ptr,
            marker: PhantomData,
            a: a,
        }
    }
}

#[cfg(feature = "heap")]
impl<T> Box<T> {
    /// Allocates memory on the heap and then places `x` into it.
    ///
    /// This doesn't actually allocate if `T` is zero-sized.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate allocator_api;
    /// use allocator_api::Box;
    /// # fn main() {
    /// let five = Box::new(5);
    /// # }
    /// ```
    #[inline(always)]
    pub fn new(x: T) -> Box<T> {
        Box::new_in(x, Heap)
    }
}

#[cfg(feature = "heap")]
impl<T: ?Sized> Box<T> {
    /// Constructs a box from a raw pointer.
    ///
    /// After calling this function, the raw pointer is owned by the
    /// resulting `Box`. Specifically, the `Box` destructor will call
    /// the destructor of `T` and free the allocated memory. Since the
    /// way `Box` allocates and releases memory is unspecified, the
    /// only valid pointer to pass to this function is the one taken
    /// from another `Box` via the [`Box::into_raw`] function.
    ///
    /// This function is unsafe because improper use may lead to
    /// memory problems. For example, a double-free may occur if the
    /// function is called twice on the same raw pointer.
    ///
    /// [`Box::into_raw`]: struct.Box.html#method.into_raw
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate allocator_api;
    /// use allocator_api::Box;
    /// # fn main() {
    /// let x = Box::new(5);
    /// let ptr = Box::into_raw(x);
    /// let x = unsafe { Box::from_raw(ptr) };
    /// # }
    /// ```
    #[inline]
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Box {
            ptr: NonNull::new_unchecked(raw),
            marker: PhantomData,
            a: Heap,
        }
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
    /// extern crate allocator_api;
    /// use allocator_api::Box;
    /// # include!("dummy.rs");
    /// # fn main() {
    /// let x = Box::new_in(5, MyHeap);
    /// let ptr = Box::into_raw(x);
    /// let x = unsafe { Box::from_raw_in(ptr, MyHeap) };
    /// # }
    /// ```
    #[inline]
    pub unsafe fn from_raw_in(raw: *mut T, a: A) -> Self {
        Box {
            ptr: NonNull::new_unchecked(raw),
            marker: PhantomData,
            a: a,
        }
    }

    /// Consumes the `Box`, returning the wrapped raw pointer.
    ///
    /// After calling this function, the caller is responsible for the
    /// memory previously managed by the `Box`. In particular, the
    /// caller should properly destroy `T` and release the memory. The
    /// proper way to do so is to convert the raw pointer back into a
    /// `Box` with the [`Box::from_raw`] or the [`Box::from_raw_in`]
    /// functions, with the appropriate allocator.
    ///
    /// Note: this is an associated function, which means that you have
    /// to call it as `Box::into_raw(b)` instead of `b.into_raw()`. This
    /// is so that there is no conflict with a method on the inner type.
    ///
    /// [`Box::from_raw`]: struct.Box.html#method.from_raw
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate allocator_api;
    /// use allocator_api::Box;
    /// # include!("dummy.rs");
    /// # fn main() {
    /// let x = Box::new_in(5, MyHeap);
    /// let ptr = Box::into_raw(x);
    /// # }
    /// ```
    #[inline]
    pub fn into_raw(b: Box<T, A>) -> *mut T {
        let ptr = b.ptr.as_ptr();
        mem::forget(b);
        ptr
    }

    /// Consumes and leaks the `Box`, returning a mutable reference,
    /// `&'a mut T`. Here, the lifetime `'a` may be chosen to be `'static`.
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
    /// extern crate allocator_api;
    /// use allocator_api::Box;
    /// # include!("dummy.rs");
    /// # fn main() {
    /// let x = Box::new_in(41, MyHeap);
    /// let static_ref: &'static mut usize = Box::leak(x);
    /// *static_ref += 1;
    /// assert_eq!(*static_ref, 42);
    /// # }
    /// ```
    ///
    /// Unsized data:
    ///
    /// ```
    /// extern crate allocator_api;
    /// # use std::ptr;
    /// # include!("dummy.rs");
    /// use allocator_api::{Alloc, Box, RawVec};
    /// struct MyVec<T, A: Alloc> {
    ///     buf: RawVec<T, A>,
    ///     len: usize,
    /// }
    ///
    /// impl<T, A: Alloc> MyVec<T, A> {
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
    ///     let mut v = MyVec { buf: RawVec::new_in(MyHeap), len: 0 };
    ///     v.push(1);
    ///     v.push(2);
    ///     v.push(3);
    ///     v.buf.shrink_to_fit(v.len);
    ///     let x = unsafe { v.buf.into_box() };
    ///     let static_ref = Box::leak(x);
    ///     static_ref[0] = 4;
    ///     assert_eq!(*static_ref, [4, 2, 3]);
    /// }
    /// ```
    #[inline]
    pub fn leak<'a>(b: Box<T, A>) -> &'a mut T
    where
        T: 'a // Technically not needed, but kept to be explicit.
    {
        unsafe { &mut *Box::into_raw(b) }
    }
}

impl<T: ?Sized, A: Alloc> Drop for Box<T, A> {
    fn drop(&mut self) {
        unsafe {
            let value = self.ptr.as_ref();
            if mem::size_of_val(value) != 0 {
                let layout = Layout::for_value(value);
                self.a.dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

#[cfg(feature = "heap")]
impl<T: Default> Default for Box<T> {
    /// Creates a `Box<T>`, with the `Default` value for T.
    fn default() -> Box<T> {
        Box::new(Default::default())
    }
}

#[cfg(feature = "heap")]
impl<T> Default for Box<[T]> {
    fn default() -> Box<[T]> {
        let raw = Box::into_raw(Box::<[T; 0]>::new([]));
        unsafe { Box::from_raw(raw) }
    }
}

impl<T: Clone, A: Alloc + Clone> Clone for Box<T, A> {
    /// Returns a new box with a `clone()` of this box's contents.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate allocator_api;
    /// use allocator_api::Box;
    /// # include!("dummy.rs");
    /// # fn main() {
    /// let x = Box::new_in(5, MyHeap);
    /// let y = x.clone();
    /// # }
    /// ```
    #[inline]
    fn clone(&self) -> Box<T, A> {
        Box::new_in((**self).clone(), self.a.clone())
    }
    /// Copies `source`'s contents into `self` without creating a new allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate allocator_api;
    /// use allocator_api::Box;
    /// # include!("dummy.rs");
    /// # fn main() {
    /// let x = Box::new_in(5, MyHeap);
    /// let mut y = Box::new_in(10, MyHeap);
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
    #[cfg(feature = "i128")]
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
    #[cfg(feature = "i128")]
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i)
    }
}

#[cfg(feature = "heap")]
impl<T> From<T> for Box<T> {
    fn from(t: T) -> Self {
        Box::new(t)
    }
}

#[cfg(feature = "heap")]
impl<'a, T: Copy> From<&'a [T]> for Box<[T]> {
    fn from(slice: &'a [T]) -> Box<[T]> {
        let mut boxed = unsafe { RawVec::with_capacity(slice.len()).into_box() };
        boxed.copy_from_slice(slice);
        boxed
    }
}

impl<T: fmt::Display + ?Sized, A: Alloc> fmt::Display for Box<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug + ?Sized, A: Alloc> fmt::Debug for Box<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized, A: Alloc> fmt::Pointer for Box<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // It's not possible to extract the inner Uniq directly from the Box,
        // instead we cast it to a *const which aliases the Unique
        let ptr: *const T = &**self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl<T: ?Sized, A: Alloc> Deref for Box<T, A> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized, A: Alloc> DerefMut for Box<T, A> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
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
}

impl<I: ExactSizeIterator + ?Sized, A: Alloc> ExactSizeIterator for Box<I, A> {
    fn len(&self) -> usize {
        (**self).len()
    }
}

#[cfg(feature = "fused")]
impl<I: FusedIterator + ?Sized, A: Alloc> FusedIterator for Box<I, A> {}

impl<T: Clone, A: Alloc + Clone> Clone for Box<[T], A> {
    fn clone(&self) -> Self {
        let mut new = BoxBuilder {
            data: RawVec::with_capacity_in(self.len(), self.a.clone()),
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
                let max = unsafe { data.offset(self.len as isize) };

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
