use chumsky::error::Rich;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot parse search emails query `{1}`")]
    ParseError(Vec<Rich<'static, char>>, String),
}
