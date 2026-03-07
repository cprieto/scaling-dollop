pub mod dbt;
pub mod fpt;

use crate::errors::Error;

/// Reads a memo field
pub trait MemoRead {
    fn read_memo(&mut self, index: u32) -> Result<Vec<u8>, Error>;
    fn next_available_block(&self) -> u32;
}

/// Represent a value from a memo field
pub trait FromMemo: Sized {
    fn from_memo(raw: Vec<u8>) -> Result<Self, Error>;
}

impl FromMemo for String {
    fn from_memo(raw: Vec<u8>) -> Result<Self, Error> {
        String::from_utf8(raw).map_err(|_| Error::Conversion)
    }
}

impl FromMemo for Vec<u8> {
    fn from_memo(raw: Vec<u8>) -> Result<Self, Error> {
        Ok(raw)
    }
}
