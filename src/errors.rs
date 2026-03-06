use std::io;
use thiserror::Error;
use crate::dbf::FieldType;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unable to convert to type")]
    Conversion,
    #[error("file format error: {0}")]
    FileFormat(String),
    #[error("field {0} do not exist")]
    FieldNotFound(String),
    #[error("feature not supported")]
    NotSupported,
    #[error("invalid value for field: {0}")]
    Fieldvalue(String),
}
