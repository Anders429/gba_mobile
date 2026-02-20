//! Custom ArrayVec implementation.
//!
//! The main difference here is the size of the length. We use a single byte instead of a larger
//! `usize`. In practice, this means that we potentially save multiple bytes (depending on the
//! alignment). This is a good tradeoff to make, considering we never store more than `u8::MAX`
//! bytes and we are usually storing bytes directly.

pub(crate) mod error;

use core::{
    fmt,
    fmt::{Debug, Formatter},
    mem::MaybeUninit,
    ptr, slice,
};

pub(crate) struct ArrayVec<T, const CAP: usize> {
    len: u8,
    data: [MaybeUninit<T>; CAP],
}

impl<T, const CAP: usize> ArrayVec<T, CAP> {
    pub(crate) const fn new() -> Self {
        assert!(
            CAP <= u8::MAX as usize,
            "largest supported capacity is u8::MAX"
        );

        Self {
            len: 0,
            data: [const { MaybeUninit::uninit() }; CAP],
        }
    }

    pub(crate) fn try_from_iter<IntoIter>(into_iter: IntoIter) -> Result<Self, error::Capacity<CAP>>
    where
        IntoIter: IntoIterator<Item = T>,
    {
        let mut arrayvec = ArrayVec::new();

        into_iter
            .into_iter()
            .map(|element| arrayvec.try_push(element))
            .collect::<Result<(), error::Capacity<CAP>>>()?;

        Ok(arrayvec)
    }

    pub(crate) const fn len(&self) -> u8 {
        self.len
    }

    pub(crate) fn get(&self, index: u8) -> Option<&T> {
        if index < self.len() {
            Some(unsafe { self.data.get_unchecked(index as usize).assume_init_ref() })
        } else {
            None
        }
    }

    const fn as_ptr(&self) -> *const T {
        self.data.as_ptr() as _
    }

    const fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr() as _
    }

    const fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len() as usize) }
    }

    fn try_push(&mut self, element: T) -> Result<(), error::Capacity<CAP>> {
        let len = self.len() as usize;
        if len < CAP {
            unsafe {
                ptr::write(self.as_mut_ptr().add(len), element);
            }
            self.len += 1;
            Ok(())
        } else {
            Err(error::Capacity)
        }
    }
}

// TODO: Do we need to keep this implementation around?
impl<T, const CAP: usize> Clone for ArrayVec<T, CAP>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        // Clone only the data that we know is initialized.
        let mut data = [const { MaybeUninit::uninit() }; CAP];
        for (i, element) in self
            .data
            .iter()
            .take(self.len() as usize)
            .map(|element| unsafe { element.assume_init_ref() })
            .enumerate()
        {
            data[i] = MaybeUninit::new(element.clone())
        }

        Self {
            len: self.len,
            data,
        }
    }
}

impl<T, const CAP: usize> Debug for ArrayVec<T, CAP>
where
    T: Debug,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.as_slice().fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::{ArrayVec, error};
    use alloc::format;
    use claims::{assert_err_eq, assert_ok};
    use core::mem;
    use gba_test::test;

    #[test]
    fn size() {
        assert_eq!(mem::size_of::<ArrayVec<u8, 255>>(), 256);
    }

    #[test]
    fn align_1() {
        assert_eq!(mem::align_of::<ArrayVec<u8, 255>>(), 1);
    }

    #[test]
    fn align_4() {
        assert_eq!(mem::align_of::<ArrayVec<u32, 255>>(), 4);
    }

    #[test]
    fn try_push_success() {
        let mut arrayvec: ArrayVec<u8, 1> = ArrayVec::new();

        assert_ok!(arrayvec.try_push(42));
    }

    #[test]
    fn try_push_error() {
        let mut arrayvec: ArrayVec<u8, 0> = ArrayVec::new();

        assert_err_eq!(arrayvec.try_push(42), error::Capacity);
    }

    #[test]
    fn try_from_iter_success() {
        assert_ok!(ArrayVec::<u8, 5>::try_from_iter([1, 2, 3, 4, 5]));
    }

    #[test]
    fn try_from_iter_error() {
        assert_err_eq!(
            ArrayVec::<u8, 4>::try_from_iter([1, 2, 3, 4, 5]),
            error::Capacity
        );
    }

    #[test]
    fn len_empty() {
        let arrayvec: ArrayVec<u16, 8> = ArrayVec::new();

        assert_eq!(arrayvec.len(), 0);
    }

    #[test]
    fn len_nonempty() {
        let arrayvec: ArrayVec<u16, 8> = assert_ok!(ArrayVec::try_from_iter([1, 2, 3]));

        assert_eq!(arrayvec.len(), 3);
    }

    #[test]
    fn debug_empty() {
        let arrayvec: ArrayVec<bool, 0> = ArrayVec::new();

        assert_eq!(format!("{arrayvec:?}"), "[]");
    }

    #[test]
    fn debug_nonempty() {
        let arrayvec: ArrayVec<bool, 3> = assert_ok!(ArrayVec::try_from_iter([true, false, false]));

        assert_eq!(format!("{arrayvec:?}"), "[true, false, false]");
    }
}
