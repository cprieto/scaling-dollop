use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};

fn main() -> anyhow::Result<()> {
    let mut file = File::open("BLAH.DBT")?;
    let x = file.read_u32::<LittleEndian>()?;

    println!("pos: {x}");
    file.seek(SeekFrom::Start(16))?;
    let x = file.read_u8()?;
    println!("pos: {x}");

    Ok(())
}