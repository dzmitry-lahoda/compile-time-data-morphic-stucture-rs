use std::{alloc, mem::ManuallyDrop};

use smallvec::SmallVec;

/// packed
union VecData<const U: usize, T> {
    pub small: ManuallyDrop<smallvec::SmallVec<[T; U]>>,
    pub big: ManuallyDrop<std::vec::Vec<T>>,
}

pub struct Vec<const A: usize, T> {
    data: VecData<A, T>,
}

impl<const A: usize, T> Vec<A, T> {
    pub fn new() -> Self {
        if A > 8 {
            Self {
                data: VecData {
                    big: ManuallyDrop::new(std::vec::Vec::new()),
                },
            }
        } else {
            Self {
                data: VecData {
                    small: ManuallyDrop::new(SmallVec::new()),
                },
            }
        }
    }
}
