use crate::dbf::header::{DbfVersion, Field, Header};
use crate::dbf::row::Rows;
use crate::errors::Error;
use crate::errors::Error::FileFormat;
use crate::memo::MemoRead;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use time::{Date, Month};

/// A DBF table reader
/// used to get fields and rows from a DBF
pub struct DbfReader<R: Read + Seek> {
    reader: R,
    memo: Option<Box<dyn MemoRead>>,
    header: Header,
    fields: Arc<Vec<Field>>,
}

const FIELD_START: u64 = 32;
const FIELD_SIZE: u64 = 32;

impl<R: Read + Seek> DbfReader<R> {
    /// Creates a DBF parser from a reader
    pub fn from_reader(mut reader: R) -> Result<Self, Error> {
        reader.seek(SeekFrom::Start(0))?;
        let version = reader.read_u8()?;
        let version = DbfVersion::from_repr(version)
            .ok_or(FileFormat(format!("invalid file version: {version}")))?;

        let year = reader.read_u8()?;
        let year = 1900 + (year as i32);

        let month = reader.read_u8()?;
        let month = Month::try_from(month)
            .map_err(|_| FileFormat(format!("invalid month in file header: {month}")))?;

        let day = reader.read_u8()?;

        let last_update = Date::from_calendar_date(year, month, day)
            .map_err(|_| FileFormat(format!("invalid date in header: {year}.{month}.{day}")))?;

        let num_records = reader.read_u32::<LittleEndian>()?;

        let record_start = reader.read_u16::<LittleEndian>()?;

        let record_length = reader.read_u16::<LittleEndian>()?;

        let mut fields = Vec::new();
        let mut loc = 0;
        let mut offset = 1;
        loop {
            let pos = FIELD_START + FIELD_SIZE * loc;
            reader.seek(SeekFrom::Start(pos))?;

            // maybe there are no more fields?
            let terminator = reader.read_u8()?;
            if terminator == 0x0d {
                break;
            }

            reader.seek(SeekFrom::Start(pos))?;
            let field = Field::new(&mut reader, offset)?;
            offset += field.size();
            fields.push(field);

            loc += 1;
        }

        let header = Header {
            version,
            last_update,
            num_records,
            record_start,
            record_length,
        };

        Ok(Self {
            reader,
            header,
            memo: None,
            fields: Arc::new(fields),
        })
    }

    /// Sets a memo reader for memo fields
    pub fn with_memo(mut self, memo: impl MemoRead + 'static) -> Self {
        self.memo = Some(Box::new(memo));
        self
    }

    /// Fields defined in this DBF table
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Returns iterator to rows in the DBF table
    /// this includes deleted rows
    /// only one iterator at a time!
    #[must_use]
    pub fn rows(&mut self) -> Rows<'_, R> {
        Rows::new(
            &mut self.reader,
            self.header.record_length,
            self.header.record_start,
            self.header.num_records,
            Arc::clone(&self.fields),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::dbf::header::DbfVersion;
    use crate::dbf::reader::DbfReader;
    use crate::dbf::row::Value;
    use crate::sample_file;
    use time::{Date, Month};

    #[test]
    fn dbase3_is_not_y2k_ready() -> anyhow::Result<()> {
        let mut reader = sample_file("db3.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;

        // dbase3 thinks is 1900!
        assert_eq!(
            dbf.header.last_update,
            Date::from_calendar_date(1926, Month::February, 23)?
        );

        // same with FoxPro 1.0, 2.0 and even Visual FoxPro, they are broken :(
        let mut reader = sample_file("fox1.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(
            dbf.header.last_update,
            Date::from_calendar_date(1926, Month::February, 18)?
        );

        let mut reader = sample_file("fox2.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(
            dbf.header.last_update,
            Date::from_calendar_date(1926, Month::February, 22)?
        );

        let mut reader = sample_file("vfp.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(
            dbf.header.last_update,
            Date::from_calendar_date(1926, Month::February, 23)?
        );

        // But with DB4 and after we are clean!
        let mut reader = sample_file("db4.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(
            dbf.header.last_update,
            Date::from_calendar_date(2026, Month::February, 16)?
        );

        let mut reader = sample_file("db5.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(
            dbf.header.last_update,
            Date::from_calendar_date(2026, Month::February, 17)?
        );

        Ok(())
    }

    #[test]
    fn dbase_are_all_dbase3() -> anyhow::Result<()> {
        let mut reader = sample_file("db3.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::Dbase, dbf.header.version);

        let mut reader = sample_file("db4.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::Dbase, dbf.header.version);

        let mut reader = sample_file("db5.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::Dbase, dbf.header.version);

        // same as FoxPro 1.0
        let mut reader = sample_file("fox1.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::Dbase, dbf.header.version);

        // and FoxPro 2.0
        let mut reader = sample_file("fox2.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::Dbase, dbf.header.version);

        Ok(())
    }

    #[test]
    fn vfp_is_its_own_type() -> anyhow::Result<()> {
        let mut reader = sample_file("vfp.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::VisualFoxPro, dbf.header.version);

        Ok(())
    }

    #[test]
    fn vfp_type_includes_memo() -> anyhow::Result<()> {
        let mut reader = sample_file("vfpmemo.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::VisualFoxPro, dbf.header.version);

        Ok(())
    }

    #[test]
    fn other_header_properties() -> anyhow::Result<()> {
        let mut reader = sample_file("db3.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;

        assert_eq!(8, dbf.header.num_records);
        assert_eq!(0xC1, dbf.header.record_start);
        assert_eq!(0x2E, dbf.header.record_length);

        Ok(())
    }

    #[test]
    fn read_field_types() -> anyhow::Result<()> {
        let mut reader = sample_file("db3.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;

        let fields = dbf.fields();
        assert_eq!(5, fields.len());

        Ok(())
    }

    #[test]
    fn read_rows_dbf() -> anyhow::Result<()> {
        let mut reader = sample_file("db3.dbf")?;
        let mut dbf = DbfReader::from_reader(&mut reader)?;

        let mut rows = dbf.rows();
        // we can read two rows
        let row = rows.next();
        assert!(matches!(row, Some(Ok(_))));

        let row = rows.next();
        assert!(matches!(row, Some(Ok(_))));

        // let's restart and grab all rows
        let mut records = vec![];
        for item in dbf.rows() {
            let item = item?;
            let value = item.get("NAME")?;

            records.push(value);
        }

        let expected = vec![
            Value::Character("Widget Pro".to_string()),
            Value::Character("Gadget Mini".to_string()),
            Value::Character("Thingamajig".to_string()),
            Value::Character("Doohickey XL".to_string()),
            Value::Character("Sprocket S".to_string()),
            Value::Character("Old Product".to_string()),
            Value::Character("Broken Item".to_string()),
            // At the end of this file, there is an empty record
            Value::Null,
        ];

        assert_eq!(expected, records);

        Ok(())
    }
}
