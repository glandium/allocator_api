#![no_std]
#![cfg_attr(feature = "unstable-rust", feature(alloc, allocator_api))]

#[cfg(feature = "unstable-rust")]
extern crate alloc;
#[cfg(feature = "unstable-rust")]
extern crate std;

pub mod allocator;
#[cfg(feature = "box")]
pub mod boxed;
#[cfg(feature = "unstable-rust")]
pub mod compat;
pub mod raw_vec;

pub use allocator::*;
#[cfg(feature = "box")]
pub use boxed::*;
pub use raw_vec::*;
