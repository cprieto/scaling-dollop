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
    fn test_grab_next_block_from_fpt() -> anyhow::Result<()> {
        let mut file = sample_file("foxpro_memo.fpt")?;
        let reader = FptReader::from_reader(&mut file)?;

        assert_eq!(11, reader.next_available_block());

        Ok(())
    }

    #[test]
    fn test_can_read_text_from_fpt() -> anyhow::Result<()> {
        let mut file = sample_file("foxpro_memo.fpt")?;
        let mut reader = FptReader::from_reader(&mut file)?;
        let block_size = reader.block_size as u32;
        let next_block = reader.next_block;

        // FPT has a block size of 64
        assert_eq!(64, block_size);
        // this means first block is at 8
        assert_eq!(0x0b, next_block);

        // we know we have data at block 10
        let text: String = reader.read_memo(10)?;

        assert_eq!("another memo field", &text);
        Ok(())
    }

    #[test]
    fn test_read_foxpro2_memo() -> anyhow::Result<()> {
        let mut file = sample_file("foxpro2_memo.fpt")?;
        let mut reader = FptReader::from_reader(&mut file)?;
        let block_size = reader.block_size;

        // FoxPro 2 has a 64 block size by default
        // but header is still 512
        assert_eq!(64, block_size);

        // this means first block is at 8!
        let text: String = reader.read_memo(8)?;
        assert_eq!("this is a simple memo", text);

        Ok(())
    }

    #[test]
    fn test_read_visual_foxpro() -> anyhow::Result<()> {
        let mut file = sample_file("vfp_memo.fpt")?;
        let mut reader = FptReader::from_reader(&mut file)?;
        let block_size = reader.block_size;

        assert_eq!(64, block_size);

        // first block
        let text: String = reader.read_memo(8)?;
        assert_eq!("hello world", text);

        Ok(())
    }
}
