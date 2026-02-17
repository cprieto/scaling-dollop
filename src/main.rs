use byteorder::ReadBytesExt;
use std::fs::File;

fn main() -> anyhow::Result<()> {
    let mut file = File::open("samples/db4memo.dbf")?;
    let signature = file.read_u8()?;

    println!("signature: {signature:#04x}");

    Ok(())
}
