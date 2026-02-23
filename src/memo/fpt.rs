use crate::errors::Error;
use crate::memo::{FromMemo, MemoReader};
use byteorder::{BigEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

pub struct FptReader<'a, R: Read + Seek> {
    reader: &'a mut R,
    block_size: u32,
    next_block: u32,
}

impl<'a, R: Read + Seek> MemoReader<'a, R> for FptReader<'a, R> {
    fn from_reader(reader: &'a mut R) -> Result<Self, Error> {
        let next_block = reader.read_u32::<BigEndian>()?;
        reader.seek(SeekFrom::Current(2))?;
        let block_size = reader.read_u16::<BigEndian>()? as u32;

        Ok(Self {
            reader,
            next_block,
            block_size,
        })
    }

    fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error> {
        let position = (self.block_size as u64) * (index as u64);
        self.reader.seek(SeekFrom::Start(position))?;

        let _record_type = self.reader.read_u32::<BigEndian>()?;
        let record_length = self.reader.read_u32::<BigEndian>()?;

        let mut data = Vec::with_capacity(record_length as usize);
        self.reader
            .take(record_length as u64)
            .read_to_end(&mut data)?;

        T::from_memo(data)
    }

    fn next_available_block(&self) -> u32 {
        self.next_block
    }
}

#[cfg(test)]
mod tests {
    use crate::memo::fpt::FptReader;
    use crate::memo::MemoReader;
    use crate::sample_file;

    #[test]
    fn test_fpt1_header() -> anyhow::Result<()> {
        let mut file = sample_file("fox1memo.fpt")?;
        let reader = FptReader::from_reader(&mut file)?;

        assert_eq!(13, reader.next_available_block());
        assert_eq!(64, reader.block_size);

        Ok(())
    }

    #[test]
    fn test_fpt2_header() -> anyhow::Result<()> {
        let mut file = sample_file("fox2memo.fpt")?;
        let reader = FptReader::from_reader(&mut file)?;

        assert_eq!(13, reader.next_available_block());
        assert_eq!(64, reader.block_size);

        Ok(())
    }

    #[test]
    fn test_fpt_vpf_header() -> anyhow::Result<()> {
        let mut file = sample_file("vfpmemo.fpt")?;
        let reader = FptReader::from_reader(&mut file)?;

        assert_eq!(13, reader.next_available_block());
        assert_eq!(64, reader.block_size);

        Ok(())
    }
}
