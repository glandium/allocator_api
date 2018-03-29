#![no_std]
#![cfg_attr(feature = "heap", feature(alloc, allocator_api))]

#[cfg(feature = "heap")]
extern crate alloc;
#[cfg(feature = "heap")]
extern crate std;

pub mod allocator;
#[cfg(feature = "heap")]
pub mod heap;
pub mod raw_vec;

pub use allocator::*;
#[cfg(feature = "heap")]
pub use heap::*;
pub use raw_vec::*;
