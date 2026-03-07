use scaling_dollop::dbf::reader::DbfReader;
use std::fs::File;

fn main() -> anyhow::Result<()> {
    let file = File::open("samples/vfp.dbf")?;

    let reader = DbfReader::from_reader(file)?;
    for field in reader.fields() {
        println!("{}: {:?}", field.name(), field.field_type());
    }

    Ok(())
}
