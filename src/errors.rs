use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unable to convert to type")]
    Conversion,
    #[error("file format error: {0}")]
    FileFormat(String),
}
