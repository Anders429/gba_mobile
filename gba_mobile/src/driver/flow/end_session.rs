use crate::{
    Timer,
    driver::{Request, Source},
    mmio::serial::TransferLength,
};
use core::{
    fmt,
    fmt::{Debug, Formatter},
    mem::transmute,
};

/// The next state to switch to after the session has been ended.
#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::driver) enum Destination {
    NotConnected = 0,
    LinkingP2P = 1,
}

/// The actual flow state for ending the session.
#[derive(Debug, Eq, PartialEq)]
#[repr(u8)]
enum Flow {
    /// Send the end session command.
    EndSession = 0,
    /// Wait for idle to be returned.
    ///
    /// We must do this because the adapter will switch from SIO32 to SIO8.
    WaitForIdle = 1,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub(in crate::driver) struct EndSession(u8);

impl EndSession {
    pub(in crate::driver) fn new(destination: Destination) -> Self {
        Self((destination as u8) << 4)
    }

    pub(in crate::driver) fn destination(self) -> Destination {
        unsafe { transmute((self.0 & 0b0001_0000) >> 4) }
    }

    pub(in crate::driver) fn set_destination(self, destination: Destination) -> Self {
        Self((self.0 & 0b1110_1111) | ((destination as u8) << 4))
    }

    fn flow(self) -> Flow {
        unsafe { transmute(self.0 & 0b0000_0001) }
    }

    pub(in crate::driver) fn request(
        self,
        timer: Timer,
        transfer_length: TransferLength,
    ) -> Request {
        match self.flow() {
            Flow::EndSession => Request::new_packet(timer, transfer_length, Source::EndSession),
            Flow::WaitForIdle => Request::new_wait_for_idle(),
        }
    }

    pub(in crate::driver) fn next(self) -> Option<Self> {
        match self.flow() {
            Flow::EndSession => Some(Self((self.0 & 0b1111_1110) | (Flow::WaitForIdle as u8))),
            Flow::WaitForIdle => None,
        }
    }
}

impl Debug for EndSession {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("EndSession")
            .field("destination", &self.destination())
            .field("flow", &self.flow())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{Destination, EndSession, Flow};
    use alloc::format;
    use claims::assert_some;
    use gba_test::test;

    #[test]
    fn destination_not_connected() {
        let end_session = EndSession::new(Destination::NotConnected);

        assert_eq!(end_session.destination(), Destination::NotConnected);
    }

    #[test]
    fn destination_linking_p2p() {
        let end_session = EndSession::new(Destination::LinkingP2P);

        assert_eq!(end_session.destination(), Destination::LinkingP2P);
    }

    #[test]
    fn set_destination_not_connected() {
        let end_session =
            EndSession::new(Destination::LinkingP2P).set_destination(Destination::NotConnected);

        assert_eq!(end_session.destination(), Destination::NotConnected);
    }

    #[test]
    fn set_destination_linking_p2p() {
        let end_session =
            EndSession::new(Destination::NotConnected).set_destination(Destination::LinkingP2P);

        assert_eq!(end_session.destination(), Destination::LinkingP2P);
    }

    #[test]
    fn flow_end_session() {
        let end_session = EndSession::new(Destination::NotConnected);

        assert_eq!(end_session.flow(), Flow::EndSession);
    }

    #[test]
    fn flow_wait_for_idle() {
        let end_session = assert_some!(EndSession::new(Destination::NotConnected).next());

        assert_eq!(end_session.flow(), Flow::WaitForIdle);
    }

    #[test]
    fn debug() {
        let end_session = EndSession::new(Destination::NotConnected);

        assert_eq!(
            format!("{end_session:?}"),
            "EndSession { destination: NotConnected, flow: EndSession }"
        );
    }
}
