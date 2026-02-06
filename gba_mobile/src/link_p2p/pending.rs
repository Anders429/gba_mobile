use crate::{
    Engine,
    link_p2p::{Error, LinkP2P},
};

#[derive(Debug)]
pub struct Pending {}

impl Pending {
    pub fn status(&self, engine: &Engine) -> Result<Option<LinkP2P>, Error> {
        engine
            .link_p2p_status()
            .map(|finished| finished.then(|| LinkP2P {}))
            .map_err(|error| error.into())
    }
}
