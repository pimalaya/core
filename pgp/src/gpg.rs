use thiserror::Error;

use crate::Result;

#[derive(Debug, Error)]
pub enum Error {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpGpg {}

impl PgpGpg {
    pub fn sign(&self, data: &[u8], sender: impl ToString) -> Result<Vec<u8>> {
        unimplemented!()
    }
}
