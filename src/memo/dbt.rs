use std::io::{Read, Seek, SeekFrom};
use crate::errors::Error;
use crate::memo::{FromMemo, MemoReader};
use crate::read_until_terminator;
use byteorder::{LittleEndian, ReadBytesExt};

const BLOCK_SIZE: u64 = 512;

struct DbtReader<'a, R: Read + Seek> {
    next_block: u32,
    reader: &'a mut R,
}

impl<'a, R: Read + Seek> DbtReader<'a, R> {
    pub fn new(reader: &'a mut R) -> Result<Self, Error> {
        reader.seek(SeekFrom::Start(0))?;
        let next_block = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            next_block,
            reader
        })
    }
}

impl<'a, R> MemoReader for DbtReader<'a, R>
where R : Read + Seek {
    fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error> {
        let position = BLOCK_SIZE * (index as u64);
        self.reader.seek(SeekFrom::Start(position))?;
        let data = read_until_terminator(&mut self.reader, &[0x1a, 0x1a])?;

        Ok(T::from_memo(data)?)
    }

    fn next_available_block(&self) -> u32 {
        self.next_block
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::memo::dbt::DbtReader;
    use crate::memo::MemoReader;

    #[test]
    fn test_simple_dbt_header() -> anyhow::Result<()> {
        let mut file = [0; 512];
        // first block is 1 (little endian)
        file[0] = 1;
        // version is at position 16
        file[16] = 3;
        let mut cursor = Cursor::new(&file);
        let reader = DbtReader::new(&mut cursor)?;

        assert_eq!(1, reader.next_available_block());

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
        let mut reader = DbtReader::new(&mut cursor)?;
        let content: String = reader.read_memo(1)?;

        assert_eq!("Hola", &content);

        Ok(())
    }
}