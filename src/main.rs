use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use scaling_dollop::slice_until_terminator;

fn main() -> anyhow::Result<()> {
    let mut file = File::open("samples/fox1.dbf")?;
    file.seek(SeekFrom::Start(0x20))?;

    let mut name = [0u8; 11];
    file.read_exact(&mut name)?;

    let name = slice_until_terminator(&name, &[0]);
    let name = String::from_utf8(name)?;

    println!("field name: {name}");

    Ok(())
}
