use crate::errors::Error;
use crate::errors::Error::Conversion;
use crate::memo::{FromMemo, MemoReader};
use crate::read_until_terminator;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

const BLOCK_SIZE: u32 = 512;

pub struct Dbt3Reader<'a, R: Read + Seek> {
    next_block: u32,
    reader: &'a mut R,
}

impl<'a, R> MemoReader<'a, R> for Dbt3Reader<'a, R>
where
    R: Read + Seek,
{
    fn from_reader(reader: &'a mut R) -> Result<Self, Error> {
        reader.seek(SeekFrom::Start(0))?;
        let next_block = reader.read_u32::<LittleEndian>()?;

        Ok(Self { next_block, reader })
    }

    fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error> {
        let position = (BLOCK_SIZE as u64) * (index as u64);
        self.reader.seek(SeekFrom::Start(position))?;
        let data = read_until_terminator(&mut self.reader, &[0x1a, 0x1a])?;

        T::from_memo(data)
    }

    fn next_available_block(&self) -> u32 {
        self.next_block
    }
}

pub struct Dbt4Reader<'a, R: Read + Seek> {
    next_block: u32,
    block_size: u32,
    reader: &'a mut R,
}

impl<'a, R: Read + Seek> MemoReader<'a, R> for Dbt4Reader<'a, R> {
    fn from_reader(reader: &'a mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let next_block = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Start(20))?;
        let block_size = reader.read_u16::<LittleEndian>()? as u32;

        Ok(Self {
            next_block,
            block_size,
            reader,
        })
    }

    fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error> {
        let position = (index * self.block_size) as u64;
        self.reader.seek(SeekFrom::Start(position + 4))?;

        // grab memo length, this is total length of the field!
        let length = self.reader.read_u32::<LittleEndian>()?;
        let length = length.checked_sub(8).ok_or(Conversion)?;
        let mut output = Vec::with_capacity(length as usize);
        self.reader.take(length as u64).read_to_end(&mut output)?;

        T::from_memo(output)
    }

    fn next_available_block(&self) -> u32 {
        self.next_block
    }
}

#[cfg(test)]
mod tests {
    use crate::memo::dbt::{Dbt3Reader, Dbt4Reader};
    use crate::memo::MemoReader;
    use std::io::Cursor;
    use crate::sample_file;

    #[test]
    fn test_simple_dbt_header() -> anyhow::Result<()> {
        let mut file = [0; 512];
        // first block is 1 (little endian)
        file[0] = 1;
        // version is at position 16
        file[16] = 3;
        let mut cursor = Cursor::new(&file);
        let reader = Dbt3Reader::from_reader(&mut cursor)?;

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
        let mut reader = Dbt3Reader::from_reader(&mut cursor)?;
        let content: String = reader.read_memo(1)?;

        assert_eq!("Hola", &content);

        Ok(())
    }

    #[test]
    fn test_open_dbt4_memo() -> anyhow::Result<()> {
        let mut file = sample_file("db4_memo.dbt")?;
        let reader = Dbt4Reader::from_reader(&mut file)?;

        assert_eq!(512, reader.block_size);

        Ok(())
    }

    #[test]
    fn test_dbt4_next_block() -> anyhow::Result<()> {
        let mut file = sample_file("db4_memo.dbt")?;
        let reader = Dbt4Reader::from_reader(&mut file)?;

        assert_eq!(3, reader.next_block);

        Ok(())
    }

    #[test]
    fn test_dbt4_read_memo() -> anyhow::Result<()> {
        let mut file = sample_file("db4_memo.dbt")?;
        let mut reader = Dbt4Reader::from_reader(&mut file)?;

        let content: String = reader.read_memo(2)?;
        assert_eq!("hello world!", &content);

        Ok(())
    }
}
