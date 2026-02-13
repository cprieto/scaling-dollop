#[cfg(test)]
mod tests {
    use std::io::Cursor;

    #[test]
    fn test_simple_dbt_header() -> anyhow::Result<()> {
        // let mut file = [0; 512];
        // // first block is 1 (little endian)
        // file[0] = 1;
        // // version is at position 16
        // file[16] = 3;
        // let mut cursor = Cursor::new(&file);
        // let reader = MemoReader::from_reader(&mut cursor)?;
        //
        // // by default block is 512 and version is 03h
        // assert_eq!(512, reader.header.block_size);
        // assert_eq!(3, reader.header.version);
        // // next block is 1, 0 is header
        // assert_eq!(1, reader.header.next_block);

        Ok(())
    }

    #[test]
    fn test_extract_text_from_block() -> anyhow::Result<()> {
        // let mut file = [0; 1024];
        // file[0] = 2;
        // file[16] = 3;
        // // block 1 contains 'Hola'
        // file[512] = 72;
        // file[513] = 111;
        // file[514] = 108;
        // file[515] = 97;
        // file[516] = 0x1a;
        // file[517] = 0x1a;
        //
        // let mut cursor = Cursor::new(&file);
        // let mut reader = MemoReader::from_reader(&mut cursor)?;
        // let content: String = reader.read_memo(1)?;
        //
        // assert_eq!("Hola", &content);

        Ok(())
    }
}