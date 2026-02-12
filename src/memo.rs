use crate::errors::Error;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{ErrorKind, Read, Result as IOResult, Seek, SeekFrom};

fn read_until_terminator<R: Read>(reader: &mut R, delimiter: &[u8]) -> IOResult<Vec<u8>> {
    assert!(!delimiter.is_empty(), "delimiter must not be empty");

    let mut output = vec![];
    loop {
        match reader.read_u8() {
            Ok(b) => {
                output.push(b);
                if output.len() >= delimiter.len() {
                    let size = output.len() - delimiter.len();
                    if output[size..] == *delimiter {
                        output.truncate(size);
                        return Ok(output);
                    }
                }
            }
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(output),
            Err(err) => return Err(err),
        }
    }
}
struct MemoHeader {
    block_size: usize,
    next_block: u32,
    version: u8,
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

/// Reader for special memo streams
pub struct MemoReader<'a, R: Read + Seek> {
    reader: &'a mut R,
    header: MemoHeader,
}

impl<'a, R> MemoReader<'a, R>
where
    R: Read + Seek,
{
    // Reads a memo from a given string
    pub fn from_reader(reader: &'a mut R) -> Result<Self, Error> {
        reader.seek(SeekFrom::Start(0))?;
        let next_block = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Start(16))?;
        let version = reader.read_u8()?;

        let header = MemoHeader {
            block_size: 512,
            next_block,
            version,
        };

        Ok(Self { reader, header })
    }

    pub fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error> {
        let position = index * self.header.block_size as u32;
        self.reader.seek(SeekFrom::Start(position as u64))?;

        let result = read_until_terminator(self.reader, &[0x1a, 0x1a])?;
        T::from_memo(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::memo::{MemoReader, read_until_terminator};
    use std::io::Cursor;

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

    #[test]
    fn test_simple_dbt_header() -> anyhow::Result<()> {
        let mut file = [0; 512];
        // first block is 1 (little endian)
        file[0] = 1;
        // version is at position 16
        file[16] = 3;
        let mut cursor = Cursor::new(&file);
        let reader = MemoReader::from_reader(&mut cursor)?;

        // by default block is 512 and version is 03h
        assert_eq!(512, reader.header.block_size);
        assert_eq!(3, reader.header.version);
        // next block is 1, 0 is header
        assert_eq!(1, reader.header.next_block);

        Ok(())
    }

    #[test]
    fn test_extract_text_from_block() -> anyhow::Result<()> {
        let mut file = [0; 1024];
        file[0] = 2;
        file[16] = 3;
        // block 1 contains 'Hola'
        file[512] = 72;
        file[513] = 111;
        file[514] = 108;
        file[515] = 97;
        file[516] = 0x1a;
        file[517] = 0x1a;

        let mut cursor = Cursor::new(&file);
        let mut reader = MemoReader::from_reader(&mut cursor)?;
        let content: String = reader.read_memo(1)?;

        assert_eq!("Hola", &content);

        Ok(())
    }
}
