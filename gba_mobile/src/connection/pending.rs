use super::Connection;
use crate::{
    Driver, config, dns,
    pending::{self, Pendable, PendableError},
    socket,
};
use core::marker::PhantomData;

impl<Socket1, Socket2, Dns, Config, Mode> PendableError<Socket1, Socket2, Dns, Config>
    for Connection<Driver<Socket1, Socket2, Dns, Config>, Mode>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
    Mode: super::Mode<Socket1, Socket2, Dns, Config>,
{
    type Error = Mode::Error;
}

impl<Socket1, Socket2, Dns, Config, Mode> pending::Sealed<Socket1, Socket2, Dns, Config>
    for Connection<Driver<Socket1, Socket2, Dns, Config>, Mode>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
    Mode: super::Mode<Socket1, Socket2, Dns, Config>,
{
    type State = Self;

    fn status(
        state: &Self::State,
        driver: &Driver<Socket1, Socket2, Dns, Config>,
    ) -> Option<Result<Self, Self::Error>> {
        state
            .mode
            .status(driver, state.link_generation, state.connection_generation)
            .map(|finished| {
                finished.then(|| Connection {
                    link_generation: state.link_generation,
                    connection_generation: state.connection_generation,
                    mode: state.mode,
                    driver: PhantomData,
                })
            })
            .transpose()
    }

    fn cancel(
        state: Self::State,
        driver: &mut Driver<Socket1, Socket2, Dns, Config>,
    ) -> Result<(), Self::Error> {
        state
            .mode
            .cancel(driver, state.link_generation, state.connection_generation)
    }
}

impl<Socket1, Socket2, Dns, Config, Mode> Pendable<Socket1, Socket2, Dns, Config>
    for Connection<Driver<Socket1, Socket2, Dns, Config>, Mode>
where
    Socket1: socket::Slot,
    Socket2: socket::Slot,
    Dns: dns::Mode,
    Config: config::Mode,
    Mode: super::Mode<Socket1, Socket2, Dns, Config>,
{
}
