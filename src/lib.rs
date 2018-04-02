#![no_std]
#![cfg_attr(feature = "unstable-rust", feature(alloc, allocator_api))]

#[cfg(feature = "unstable-rust")]
extern crate alloc;
#[cfg(feature = "heap")]
#[macro_use]
extern crate std;
#[cfg(all(feature = "heap", not(feature = "unstable-rust")))]
#[macro_use]
extern crate static_assertions;

pub mod allocator;
#[cfg(feature = "box")]
pub mod boxed;
#[cfg(feature = "unstable-rust")]
pub mod compat;
#[cfg(feature = "heap")]
pub mod heap;
pub mod raw_vec;

pub use allocator::*;
#[cfg(feature = "box")]
pub use boxed::*;
#[cfg(feature = "heap")]
pub use heap::*;
pub use raw_vec::*;
