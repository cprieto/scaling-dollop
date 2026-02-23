use std::fs::File;
use std::io::{Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use scaling_dollop::{reader_until_terminator};

fn main() -> anyhow::Result<()> {
    let mut file = File::open("samples/db4memo.dbt")?;
    let next_block = file.read_u32::<LittleEndian>()?;
    println!("next block: {next_block}");

    file.seek(SeekFrom::Start(16))?;
    let version = file.read_u8()?;
    println!("version? {version}");

    file.seek(SeekFrom::Start(20))?;
    let block_size = file.read_u16::<LittleEndian>()? as u32;
    println!("block size: {block_size}");

    file.seek(SeekFrom::Start(512))?;

    let content = reader_until_terminator(&mut file, &[0x1a, 0x1a])?;

    let content = String::from_utf8_lossy(&content);

    println!("memo content: '{content}'");

    Ok(())
}
