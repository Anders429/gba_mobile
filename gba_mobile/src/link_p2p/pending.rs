use crate::{
    Engine, Generation,
    link_p2p::{Error, LinkP2P},
};

#[derive(Debug)]
pub struct Pending {
    pub(crate) generation: Generation,
}

impl Pending {
    pub fn status(&self, engine: &Engine) -> Result<Option<LinkP2P>, Error> {
        engine
            .link_p2p_status(self.generation)
            .map(|finished| {
                finished.then(|| LinkP2P {
                    generation: self.generation,
                })
            })
            .map_err(|error| error.into())
    }
}
