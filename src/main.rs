use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

fn main() -> anyhow::Result<()> {
    let mut file = File::open("samples/vfp_memo.fpt")?;
    let next = file.read_u32::<BigEndian>()?;

    println!("next memo: {next}");
    file.seek(SeekFrom::Start(6))?;
    let block_size = file.read_u16::<BigEndian>()?;
    println!("block size: {block_size}");

    // Move past to first byte of block
    file.seek(SeekFrom::Start(512))?;
    let x = file.read_u32::<BigEndian>()?;
    println!("record type: {x:#04x}");

    // Get record length
    let length = file.read_u32::<BigEndian>()?;
    println!("length: {length:#04x} ({length})");

    // Now read that length
    let mut buffer = Vec::with_capacity(length as usize);
    file.take(length as u64)
        .read_to_end(&mut buffer)?;

    println!("content: {buffer:?}");
    let buffer = String::from_utf8_lossy(&buffer);
    println!("content as string: {buffer}");

    let next_record = block_size as u32 * next;
    println!("next block would be at {next_record:#05x} ({next_record})");

    Ok(())
}
