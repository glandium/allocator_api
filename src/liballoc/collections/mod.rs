//! Collection types.

use crate::alloc::{Layout, LayoutErr};

/// The error type for `try_reserve` methods.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,
    /// The memory allocator returned an error
    AllocError {
        /// The layout of allocation request that failed
        layout: Layout,

        #[doc(hidden)]
        non_exhaustive: (),
    },
}

impl From<LayoutErr> for TryReserveError {
    #[inline]
    fn from(_: LayoutErr) -> Self {
        TryReserveError::CapacityOverflow
    }
}
