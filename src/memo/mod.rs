pub mod dbt;
mod fpt;

use crate::errors::Error;

/// Reads a memo field
pub trait MemoReader {
    fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error>;
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::read_until_terminator;

    #[test]
    fn test_read_until_terminator() -> anyhow::Result<()> {
        let input = [1, 2, 3, 4, 5, 0x1a, 0x1b];
        let mut cursor = Cursor::new(input);
        let result = read_until_terminator(&mut cursor, &[0x1a, 0x1b])?;
        let expected = vec![1, 2, 3, 4, 5];
        assert_eq!(expected, result);

        // only the full sequence is accepted
        let input = [1, 2, 3, 4, 5, 0x1a, 6, 7, 0x1a, 0x1b];
        let mut cursor = Cursor::new(input);
        let result = read_until_terminator(&mut cursor, &[0x1a, 0x1b])?;
        let expected = vec![1, 2, 3, 4, 5, 0x1a, 6, 7];
        assert_eq!(expected, result);

        // if sequence is not found, it is ok I guess
        let input = [1, 2, 3, 4, 5, 0x1a];
        let mut cursor = Cursor::new(input);
        let result = read_until_terminator(&mut cursor, &[0x1a, 0x1b])?;
        let expected = vec![1, 2, 3, 4, 5, 0x1a];
        assert_eq!(expected, result);

        Ok(())
    }
}