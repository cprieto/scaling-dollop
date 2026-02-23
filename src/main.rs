use scaling_dollop::dbf::DbfReader;
use std::fs::File;

fn main() -> anyhow::Result<()> {
    let file = File::open("samples/db3.dbf")?;

    let _reader = DbfReader::from_reader(file)?;

    Ok(())
}
