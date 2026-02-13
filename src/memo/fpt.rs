use std::io::{Read, Seek, SeekFrom};
use byteorder::{BigEndian, ReadBytesExt};
use crate::errors::Error;
use crate::memo::{FromMemo, MemoReader};

struct FptReader<'a, R: Read + Seek> {
    reader: &'a mut R,
    block_size: u32,
    next_block: u32,
}

impl<'a, R: Read + Seek> MemoReader<'a, R> for FptReader<'a, R> {
    fn from_reader(reader: &'a mut R) -> Result<Self, Error> {
        let next_block = reader.read_u32::<BigEndian>()?;
        let block_size = reader.read_u32::<BigEndian>()?;

        Ok(Self {
            reader,
            next_block,
            block_size,
        })
    }

    fn read_memo<T: FromMemo>(&mut self, index: u32) -> Result<T, Error> {
        let position = self.block_size * index;
        self.reader.seek(SeekFrom::Start(position as u64))?;

        let _record_type = self.reader.read_u32::<BigEndian>()?;
        let record_length = self.reader.read_u32::<BigEndian>()?;

        let mut data = Vec::with_capacity(record_length as usize);
        self.reader.take(record_length as u64).read_to_end(&mut data)?;

        Ok(T::from_memo(data)?)
    }

    fn next_available_block(&self) -> u32 {
        self.next_block
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use crate::memo::fpt::FptReader;
    use crate::memo::MemoReader;

    fn sample_file() -> std::io::Result<File> {
        let path = format!("{}/samples/foxpro_memo.fpt", env!("CARGO_MANIFEST_DIR"));
        File::open(path)
    }

    #[test]
    fn test_grab_next_block_from_fpt() -> anyhow::Result<()> {
        let mut file = sample_file()?;
        let reader = FptReader::from_reader(&mut file)?;

        assert_eq!(11, reader.next_available_block());

        Ok(())
    }

    #[test]
    fn test_can_read_text_from_fpt() -> anyhow::Result<()> {
        let mut file = sample_file()?;
        let mut reader = FptReader::from_reader(&mut file)?;
        let block_size = reader.block_size as u32;
        let next_block = reader.next_block;

        assert_eq!(0x40, block_size);
        assert_eq!(0x0b, next_block);

        // we know we have data at block 10
        let text: String = reader.read_memo(10)?;

        assert_eq!("another memo field", &text);
        Ok(())
    }
}