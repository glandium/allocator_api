#![no_std]
#![cfg_attr(feature = "nightly_compat", feature(alloc, allocator_api))]

#[cfg(feature = "nightly_compat")]
extern crate alloc;
#[cfg(feature = "nightly_compat")]
extern crate std;

pub mod allocator;
#[cfg(feature = "box")]
pub mod boxed;
#[cfg(feature = "nightly_compat")]
pub mod compat;
pub mod raw_vec;

pub use allocator::*;
#[cfg(feature = "box")]
pub use boxed::*;
pub use raw_vec::*;
