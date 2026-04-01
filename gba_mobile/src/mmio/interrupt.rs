use core::ops::BitOr;

pub(crate) const ENABLE: *mut Enable = 0x0400_0200 as *mut Enable;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Enable(u16);

impl Enable {
    pub(crate) const VBLANK: Self = Self(0b0000_0000_0000_0001);
    pub(crate) const TIMER0: Self = Self(0b0000_0000_0000_1000);
    pub(crate) const TIMER1: Self = Self(0b0000_0000_0001_0000);
    pub(crate) const TIMER2: Self = Self(0b0000_0000_0010_0000);
    pub(crate) const TIMER3: Self = Self(0b0000_0000_0100_0000);
    pub(crate) const SERIAL: Self = Self(0b0000_0000_1000_0000);
}

impl BitOr for Enable {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        Self(self.0 | other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Enable;
    use gba_test::test;

    #[test]
    fn bitor_empty_empty() {
        assert_eq!(Enable(0) | Enable(0), Enable(0));
    }

    #[test]
    fn bitor_empty_nonempty() {
        assert_eq!(Enable(0) | Enable::TIMER2, Enable::TIMER2);
    }

    #[test]
    fn bitor_nonempty_empty() {
        assert_eq!(Enable::SERIAL | Enable(0), Enable::SERIAL);
    }

    #[test]
    fn bitor_nonempty_nonempty() {
        assert_eq!(
            Enable::VBLANK | Enable::TIMER3,
            Enable(0b0000_0000_0100_0001)
        );
    }
}
