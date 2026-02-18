use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> anyhow::Result<()> {
    let mut file = File::open("samples/db3.dbf")?;
    file.seek(SeekFrom::Start(32))?;

    let mut name = Vec::new();
    file.take(10).read_to_end(&mut name)?;
    let name = String::from_utf8(name)?;

    println!("field name: {name}");

    Ok(())
}
