use crate::ReaderUntilTerminator;
use crate::errors::Error;
use crate::errors::Error::Conversion;
use crate::memo::{FromMemo, MemoReader};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

const BLOCK_SIZE: u32 = 512;

pub struct Dbt3Reader<R: Read + Seek> {
    next_block: u32,
    reader: R,
}

impl<R> MemoReader<R> for Dbt3Reader<R>
where
    R: Read + Seek,
{
    fn from_reader(mut reader: R) -> Result<Self, Error> {
        reader.seek(SeekFrom::Start(0))?;
        let next_block = reader.read_u32::<LittleEndian>()?;

        Ok(Self { next_block, reader })
    }

    fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error> {
        let position = (BLOCK_SIZE as u64) * (index as u64);
        self.reader.seek(SeekFrom::Start(position))?;
        let data = self.reader.read_until_terminator(&[0x1a, 0x1a])?;

        T::from_memo(data)
    }

    fn next_available_block(&self) -> u32 {
        self.next_block
    }
}

pub struct Dbt4Reader<R: Read + Seek> {
    next_block: u32,
    block_size: u32,
    reader: R,
}

impl<R: Read + Seek> MemoReader<R> for Dbt4Reader<R> {
    fn from_reader(mut reader: R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        reader.seek(SeekFrom::Start(0))?;
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
        let position = index as u64 * self.block_size as u64;
        self.reader.seek(SeekFrom::Start(position + 4))?;

        // grab memo length, this is total length of the field!
        let length = self.reader.read_u32::<LittleEndian>()?;
        let length = length.checked_sub(8).ok_or(Conversion)?;
        let mut output = Vec::with_capacity(length as usize);
        self.reader
            .by_ref()
            .take(length as u64)
            .read_to_end(&mut output)?;

        T::from_memo(output)
    }

    fn next_available_block(&self) -> u32 {
        self.next_block
    }
}

#[cfg(test)]
mod tests {
    use crate::memo::MemoReader;
    use crate::memo::dbt::{Dbt3Reader, Dbt4Reader};
    use crate::sample_file;

    #[test]
    fn test_dbt3_header() -> anyhow::Result<()> {
        let mut file = sample_file("db3memo.dbt")?;
        let reader = Dbt3Reader::from_reader(&mut file)?;

        assert_eq!(reader.next_available_block(), 5);

        Ok(())
    }

    #[test]
    fn test_dbt4_header() -> anyhow::Result<()> {
        let mut file = sample_file("db4memo.dbt")?;
        let reader = Dbt4Reader::from_reader(&mut file)?;

        assert_eq!(reader.next_available_block(), 5);

        // DB4 has variable block size, but by default is 512bytes
        assert_eq!(reader.block_size, 512);

        Ok(())
    }

    #[test]
    fn test_dbt5_header() -> anyhow::Result<()> {
        let mut file = sample_file("db5memo.dbt")?;
        let reader = Dbt4Reader::from_reader(&mut file)?;

        assert_eq!(reader.next_available_block(), 5);
        assert_eq!(reader.block_size, 512);

        Ok(())
    }
}
