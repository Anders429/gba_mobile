#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Generation(u16);

impl Generation {
    pub(crate) const fn new() -> Self {
        Self(0)
    }

    pub(crate) fn increment(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::Generation;
    use gba_test::test;

    #[test]
    fn increment() {
        let generation = Generation::new().increment();

        assert_eq!(generation, Generation(1));
    }

    #[test]
    fn increment_wrap() {
        let generation = Generation(u16::MAX).increment();

        assert_eq!(generation, Generation(0));
    }

    #[test]
    fn increment_multiple() {
        let generation = Generation(42).increment().increment();

        assert_eq!(generation, Generation(44));
    }
}
