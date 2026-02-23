use scaling_dollop::dbf::DbfReader;
use std::fs::File;

fn main() -> anyhow::Result<()> {
    let mut file = File::open("samples/db3.dbf")?;

    let reader = DbfReader::from_reader(&mut file)?;

    Ok(())
}
