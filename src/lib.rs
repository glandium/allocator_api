#![no_std]
#![cfg_attr(feature = "heap", feature(alloc, allocator_api))]

#[cfg(feature = "heap")]
extern crate alloc;

pub mod allocator;
#[cfg(feature = "heap")]
pub mod heap;

pub use allocator::*;
#[cfg(feature = "heap")]
pub use heap::*;
