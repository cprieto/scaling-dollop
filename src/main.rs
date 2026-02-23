use std::fs::File;
use scaling_dollop::dbf::DbfReader;

fn main() -> anyhow::Result<()> {
    let mut file = File::open("samples/db3.dbf")?;

    let reader = DbfReader::from_reader(&mut file)?;
    println!("fields: {}", reader.num_fields());

    Ok(())
}
