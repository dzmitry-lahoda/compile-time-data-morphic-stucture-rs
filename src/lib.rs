use std::{
    fmt,
    iter::FusedIterator,
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds},
    ptr::{self, NonNull},
    slice,
};

use smallvec::SmallVec;

union VecData<const A: usize, T> {
    small: ManuallyDrop<SmallVec<[T; A]>>,
    big: ManuallyDrop<std::vec::Vec<T>>,
}

pub struct Vec<const A: usize, const S: usize, T> {
    data: VecData<A, T>,
}

impl<const A: usize, const S: usize, T> Vec<A, S, T> {
    const IS_BIG: bool = A > S;

    #[inline]
    pub fn new() -> Self {
        if Self::IS_BIG {
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

    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        if Self::IS_BIG {
            self.big().capacity()
        } else {
            self.small().capacity()
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.as_slice().as_ptr()
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        if Self::IS_BIG {
            self.big_mut().as_mut_ptr()
        } else {
            self.small_mut().as_mut_ptr()
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        if Self::IS_BIG {
            self.big().as_slice()
        } else {
            self.small().as_slice()
        }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if Self::IS_BIG {
            self.big_mut().as_mut_slice()
        } else {
            self.small_mut().as_mut_slice()
        }
    }

    #[inline]
    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: slice::SliceIndex<[T]>,
    {
        self.as_slice().get(index)
    }

    #[inline]
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: slice::SliceIndex<[T]>,
    {
        self.as_mut_slice().get_mut(index)
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        if Self::IS_BIG {
            self.big_mut().push(value);
        } else {
            self.small_mut().push(value);
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if Self::IS_BIG {
            self.big_mut().pop()
        } else {
            self.small_mut().pop()
        }
    }

    #[inline]
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, A, S, T>
    where
        R: RangeBounds<usize>,
    {
        let len = self.len();
        let (start, end) = resolve_range(range, len);
        let drain_len = end - start;
        let tail_len = len - end;
        let ptr = self.as_mut_ptr();

        unsafe {
            self.set_len(start);
            Drain {
                vec: NonNull::from(self),
                tail_start: end,
                tail_len,
                iter: slice::from_raw_parts(ptr.add(start), drain_len).iter(),
                _marker: PhantomData,
            }
        }
    }

    #[inline(always)]
    fn small(&self) -> &SmallVec<[T; A]> {
        debug_assert!(!Self::IS_BIG);
        // SAFETY: every constructor and mover in this type initializes `small`
        // exactly when `A <= S`.
        unsafe { &self.data.small }
    }

    #[inline(always)]
    fn small_mut(&mut self) -> &mut SmallVec<[T; A]> {
        debug_assert!(!Self::IS_BIG);
        // SAFETY: same invariant as `small`; `&mut self` gives unique access.
        unsafe { &mut self.data.small }
    }

    #[inline(always)]
    fn big(&self) -> &std::vec::Vec<T> {
        debug_assert!(Self::IS_BIG);
        // SAFETY: every constructor and mover in this type initializes `big`
        // exactly when `A > S`.
        unsafe { &self.data.big }
    }

    #[inline(always)]
    fn big_mut(&mut self) -> &mut std::vec::Vec<T> {
        debug_assert!(Self::IS_BIG);
        // SAFETY: same invariant as `big`; `&mut self` gives unique access.
        unsafe { &mut self.data.big }
    }

    #[inline(always)]
    unsafe fn set_len(&mut self, len: usize) {
        if Self::IS_BIG {
            unsafe { self.big_mut().set_len(len) };
        } else {
            unsafe { self.small_mut().set_len(len) };
        }
    }
}

impl<const A: usize, const S: usize, T> Default for Vec<A, S, T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const A: usize, const S: usize, T> Drop for Vec<A, S, T> {
    fn drop(&mut self) {
        unsafe {
            if Self::IS_BIG {
                ManuallyDrop::drop(&mut self.data.big);
            } else {
                ManuallyDrop::drop(&mut self.data.small);
            }
        }
    }
}

impl<const A: usize, const S: usize, T> Clone for Vec<A, S, T>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        if Self::IS_BIG {
            Self {
                data: VecData {
                    big: ManuallyDrop::new(self.big().clone()),
                },
            }
        } else {
            Self {
                data: VecData {
                    small: ManuallyDrop::new(self.small().clone()),
                },
            }
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if Self::IS_BIG {
            self.big_mut().clone_from(source.big());
        } else {
            self.small_mut().clone_from(source.small());
        }
    }
}

impl<const A: usize, const S: usize, T> fmt::Debug for Vec<A, S, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<const A: usize, const S: usize, T, I> Index<I> for Vec<A, S, T>
where
    I: slice::SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<const A: usize, const S: usize, T, I> IndexMut<I> for Vec<A, S, T>
where
    I: slice::SliceIndex<[T]>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

impl<const A: usize, const S: usize, T> Deref for Vec<A, S, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const A: usize, const S: usize, T> DerefMut for Vec<A, S, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<const A: usize, const S: usize, T> Extend<T> for Vec<A, S, T> {
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        if Self::IS_BIG {
            self.big_mut().extend(iter);
        } else {
            self.small_mut().extend(iter);
        }
    }
}

impl<const A: usize, const S: usize, T> FromIterator<T> for Vec<A, S, T> {
    #[inline]
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut vec = Self::new();
        vec.extend(iter);
        vec
    }
}

impl<'a, const A: usize, const S: usize, T> IntoIterator for &'a Vec<A, S, T> {
    type IntoIter = slice::Iter<'a, T>;
    type Item = &'a T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, const A: usize, const S: usize, T> IntoIterator for &'a mut Vec<A, S, T> {
    type IntoIter = slice::IterMut<'a, T>;
    type Item = &'a mut T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<const A: usize, const S: usize, T> IntoIterator for Vec<A, S, T> {
    type IntoIter = IntoIter<A, S, T>;
    type Item = T;

    #[inline]
    fn into_iter(mut self) -> Self::IntoIter {
        let len = self.len();
        unsafe { self.set_len(0) };

        IntoIter {
            vec: ManuallyDrop::new(self),
            current: 0,
            end: len,
        }
    }
}

pub struct Drain<'a, const A: usize, const S: usize, T> {
    vec: NonNull<Vec<A, S, T>>,
    tail_start: usize,
    tail_len: usize,
    iter: slice::Iter<'a, T>,
    _marker: PhantomData<&'a mut Vec<A, S, T>>,
}

unsafe impl<const A: usize, const S: usize, T: Send> Send for Drain<'_, A, S, T> {}
unsafe impl<const A: usize, const S: usize, T: Sync> Sync for Drain<'_, A, S, T> {}

impl<const A: usize, const S: usize, T> Iterator for Drain<'_, A, S, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|item| unsafe { ptr::read(item) })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<const A: usize, const S: usize, T> DoubleEndedIterator for Drain<'_, A, S, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|item| unsafe { ptr::read(item) })
    }
}

impl<const A: usize, const S: usize, T> ExactSizeIterator for Drain<'_, A, S, T> {
    #[inline]
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<const A: usize, const S: usize, T> FusedIterator for Drain<'_, A, S, T> {}

impl<const A: usize, const S: usize, T> Drop for Drain<'_, A, S, T> {
    fn drop(&mut self) {
        self.for_each(drop);

        if self.tail_len == 0 {
            return;
        }

        unsafe {
            let vec = self.vec.as_mut();
            let len = vec.len();
            let ptr = vec.as_mut_ptr();

            ptr::copy(ptr.add(self.tail_start), ptr.add(len), self.tail_len);
            vec.set_len(len + self.tail_len);
        }
    }
}

pub struct IntoIter<const A: usize, const S: usize, T> {
    vec: ManuallyDrop<Vec<A, S, T>>,
    current: usize,
    end: usize,
}

impl<const A: usize, const S: usize, T> IntoIter<A, S, T> {
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        let len = self.end - self.current;
        unsafe { slice::from_raw_parts(self.vec.as_ptr().add(self.current), len) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.end - self.current;
        unsafe { slice::from_raw_parts_mut(self.vec.as_mut_ptr().add(self.current), len) }
    }
}

impl<const A: usize, const S: usize, T> Iterator for IntoIter<A, S, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            unsafe {
                let current = self.current;
                self.current += 1;
                Some(ptr::read(self.vec.as_ptr().add(current)))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end - self.current;
        (len, Some(len))
    }
}

impl<const A: usize, const S: usize, T> DoubleEndedIterator for IntoIter<A, S, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            unsafe {
                self.end -= 1;
                Some(ptr::read(self.vec.as_ptr().add(self.end)))
            }
        }
    }
}

impl<const A: usize, const S: usize, T> ExactSizeIterator for IntoIter<A, S, T> {}
impl<const A: usize, const S: usize, T> FusedIterator for IntoIter<A, S, T> {}

impl<const A: usize, const S: usize, T> Drop for IntoIter<A, S, T> {
    fn drop(&mut self) {
        struct DropVec<const A: usize, const S: usize, T>(*mut ManuallyDrop<Vec<A, S, T>>);

        impl<const A: usize, const S: usize, T> Drop for DropVec<A, S, T> {
            fn drop(&mut self) {
                unsafe { ManuallyDrop::drop(&mut *self.0) };
            }
        }

        let _drop_vec = DropVec(&mut self.vec);
        while let Some(item) = self.next() {
            drop(item);
        }
    }
}

fn resolve_range<R>(range: R, len: usize) -> (usize, usize)
where
    R: RangeBounds<usize>,
{
    let start = match range.start_bound() {
        Bound::Included(&start) => start,
        Bound::Excluded(&start) => start.checked_add(1).expect("range start index overflow"),
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&end) => end.checked_add(1).expect("range end index overflow"),
        Bound::Excluded(&end) => end,
        Bound::Unbounded => len,
    };

    assert!(
        start <= end,
        "range start index {start} out of range for slice of length {len}"
    );
    assert!(
        end <= len,
        "range end index {end} out of range for slice of length {len}"
    );

    (start, end)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::Vec;

    #[test]
    fn small_branch_behaves_like_vec() {
        let mut vec = Vec::<4, 8, i32>::new();

        assert!(vec.is_empty());
        vec.push(10);
        vec.push(20);
        vec.push(30);

        assert_eq!(vec.len(), 3);
        assert_eq!(vec.capacity(), 4);
        assert_eq!(vec.get(1), Some(&20));
        assert_eq!(vec[2], 30);
        assert_eq!(
            vec.iter().copied().collect::<std::vec::Vec<_>>(),
            [10, 20, 30]
        );

        *vec.get_mut(1).unwrap() = 25;
        assert_eq!(vec.as_slice(), &[10, 25, 30]);
        assert_eq!(vec.pop(), Some(30));

        let drained = vec.drain(..1).collect::<std::vec::Vec<_>>();
        assert_eq!(drained, [10]);
        assert_eq!(vec.as_slice(), &[25]);
    }

    #[test]
    fn big_branch_behaves_like_vec() {
        let mut vec = Vec::<16, 8, i32>::from_iter([1, 2, 3, 4]);

        assert_eq!(vec.as_slice(), &[1, 2, 3, 4]);
        vec.extend([5, 6]);
        assert_eq!(vec.as_slice(), &[1, 2, 3, 4, 5, 6]);

        for value in &mut vec {
            *value *= 2;
        }
        assert_eq!(vec.as_slice(), &[2, 4, 6, 8, 10, 12]);

        let drained = vec.drain(1..4).rev().collect::<std::vec::Vec<_>>();
        assert_eq!(drained, [8, 6, 4]);
        assert_eq!(vec.as_slice(), &[2, 10, 12]);

        let collected = vec.into_iter().collect::<std::vec::Vec<_>>();
        assert_eq!(collected, [2, 10, 12]);
    }

    #[test]
    fn clone_uses_the_active_branch() {
        let mut small = Vec::<2, 8, String>::new();
        small.push("a".to_owned());
        small.push("b".to_owned());
        let small_clone = small.clone();
        assert_eq!(small_clone.as_slice(), ["a", "b"]);

        let mut big = Vec::<16, 8, String>::new();
        big.push("x".to_owned());
        big.push("y".to_owned());
        let big_clone = big.clone();
        assert_eq!(big_clone.as_slice(), ["x", "y"]);
    }

    #[test]
    fn drain_tail_is_restored_after_partial_iteration() {
        let mut vec = Vec::<4, 8, i32>::from_iter([1, 2, 3, 4, 5, 6]);
        let mut drain = vec.drain(1..4);

        assert_eq!(drain.next(), Some(2));
        drop(drain);

        assert_eq!(vec.as_slice(), &[1, 5, 6]);
    }

    #[test]
    fn drops_elements_from_both_branches() {
        #[derive(Clone)]
        struct Counted(Rc<Cell<usize>>);

        impl Drop for Counted {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let small_drops = Rc::new(Cell::new(0));
        {
            let mut vec = Vec::<2, 8, Counted>::new();
            vec.push(Counted(Rc::clone(&small_drops)));
            vec.push(Counted(Rc::clone(&small_drops)));
        }
        assert_eq!(small_drops.get(), 2);

        let big_drops = Rc::new(Cell::new(0));
        {
            let mut vec = Vec::<16, 8, Counted>::new();
            vec.push(Counted(Rc::clone(&big_drops)));
            vec.push(Counted(Rc::clone(&big_drops)));
        }
        assert_eq!(big_drops.get(), 2);
    }
}
