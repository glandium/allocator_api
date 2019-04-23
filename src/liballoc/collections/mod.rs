//! Collection types.

use crate::alloc::{AllocErr, LayoutErr};

/// Augments `AllocErr` with a CapacityOverflow variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CollectionAllocErr {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,
    /// Error due to the allocator (see the `AllocErr` type's docs).
    AllocErr,
}

impl From<AllocErr> for CollectionAllocErr {
    #[inline]
    fn from(AllocErr: AllocErr) -> Self {
        CollectionAllocErr::AllocErr
    }
}

impl From<LayoutErr> for CollectionAllocErr {
    #[inline]
    fn from(_: LayoutErr) -> Self {
        CollectionAllocErr::CapacityOverflow
    }
}
