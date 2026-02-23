use byteorder::{LittleEndian, ReadBytesExt};
use scaling_dollop::SliceUntilTerminator;
use scaling_dollop::dbf::DbfReader;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> anyhow::Result<()> {
    let file = File::open("samples/db3.dbf")?;

    let reader = DbfReader::from_reader(file)?;

    Ok(())
}
