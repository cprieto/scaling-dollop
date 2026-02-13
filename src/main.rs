use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom};

fn main() -> anyhow::Result<()> {
    let mut file = File::open("FOXMEMO.FPT")?;
    let next = file.read_u32::<BigEndian>()?;

    println!("next memo: {next}");
    file.seek(SeekFrom::Start(6))?;
    let block_size = file.read_u16::<BigEndian>()?;
    println!("block size: {block_size}");

    // Move past to first byte of block
    file.seek(SeekFrom::Start(512))?;
    let x = file.read_u32::<BigEndian>()?;
    println!("record type: {x:#04x}");

    let next_record = block_size as u32 * next;
    println!("next block would be at {next_record:#05x}");

    Ok(())
}
