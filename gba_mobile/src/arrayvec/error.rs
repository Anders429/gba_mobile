use core::{
    fmt,
    fmt::{Display, Formatter},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Capacity<const CAP: usize>;

impl<const CAP: usize> Display for Capacity<CAP> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "insufficient capacity; maximum capacity is {CAP}"
        )
    }
}

impl<const CAP: usize> core::error::Error for Capacity<CAP> {}

#[cfg(test)]
mod tests {
    use super::Capacity;
    use alloc::format;
    use gba_test::test;

    #[test]
    fn capacity_display() {
        assert_eq!(
            format!("{}", Capacity::<42>),
            "insufficient capacity; maximum capacity is 42"
        );
    }

    #[test]
    fn capacity_display_min() {
        assert_eq!(
            format!("{}", Capacity::<0>),
            "insufficient capacity; maximum capacity is 0"
        );
    }

    #[test]
    fn capacity_display_max() {
        assert_eq!(
            format!("{}", Capacity::<255>),
            "insufficient capacity; maximum capacity is 255"
        );
    }
}
