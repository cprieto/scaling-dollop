use crate::SliceUntilTerminator;
use crate::errors::Error::{self, FileFormat};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use strum::{Display as SDisplay, FromRepr};
use time::{Date, Month};

#[derive(Debug, PartialEq, FromRepr, SDisplay)]
#[repr(u8)]
enum DbfVersion {
    #[strum(to_string = "DBase file without memo")]
    Dbase = 0x03,
    #[strum(to_string = "DBase 3 file with memo")]
    Dbase3WithMemo = 0x83,
    #[strum(to_string = "DBase 4/5 file with memo")]
    Dbase4WithMemo = 0x8b,
    #[strum(to_string = "FoxPro file with memo")]
    FoxProWithMemo = 0xf5,
    #[strum(to_string = "Visual FoxPro without memo")]
    VisualFoxPro = 0x30,
}

#[cfg_attr(not(test), expect(dead_code))]
struct Header {
    version: DbfVersion,
    last_update: Date,
    num_records: u32,
    record_start: u16,
    record_length: u16,
}

/// A DBF table reader
/// used to get fields and rows from a DBF
pub struct DbfReader<R: Read + Seek> {
    reader: R,
    header: Header,
    fields: Vec<Field>,
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
        loop {
            let pos = FIELD_START + FIELD_SIZE * loc;
            reader.seek(SeekFrom::Start(pos))?;

            // maybe there are no more fields?
            let terminator = reader.read_u8()?;
            if terminator == 0x0d {
                break;
            }

            reader.seek(SeekFrom::Start(pos))?;
            let field = Field::from_reader(&mut reader)?;
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
            fields,
        })
    }

    /// Fields defined in this DBF table
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}

/// The field (column) type and its constraints
#[derive(Clone, Copy)]
pub enum FieldType {
    Character(u8),
    Numeric { size: u8, decimal: u8 },
    Float { size: u8, decimal: u8 },
    Date,
    Logical,
    Memo,
    // Visual FoxPro
    Integer,
    Currency,
    DateTime,
    Double,
}

/// A field (column) defined in a given DBF table
pub struct Field {
    name: String,
    field_type: FieldType,
}

impl Field {
    fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut name = [0u8; 11];
        reader.read_exact(&mut name)?;
        let name = name.until_terminator(&[0]);
        let name = String::from_utf8_lossy(name);

        let field_type = reader.read_u8()?;
        reader.seek(SeekFrom::Current(4))?;

        let field_type = match field_type {
            0x43 => {
                // Read length
                let length = reader.read_u8()?;
                FieldType::Character(length)
            }
            what @ (0x4e | 0x46) => {
                // Read length and decimal places
                let size = reader.read_u8()?;
                let decimal = reader.read_u8()?;
                if what == 0x4e {
                    FieldType::Numeric { size, decimal }
                } else {
                    FieldType::Float { size, decimal }
                }
            }
            0x44 => FieldType::Date,
            0x4c => FieldType::Logical,
            0x4d => FieldType::Memo,
            0x49 => FieldType::Integer,
            0x42 => FieldType::Double,
            0x59 => FieldType::Currency,
            0x54 => FieldType::DateTime,
            _ => return Err(FileFormat(format!("invalid field type: {field_type}"))),
        };

        // While Field info is 32 bytes, we don't have much
        // to read for now, it has more info like autoincrement,
        // indices, etc...

        Ok(Self {
            name: name.into_owned(),
            field_type,
        })
    }

    /// Returns the name for this field (column)
    /// names are limited to 11 ASCII characters
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the field type for this field (column)
    pub fn field_type(&self) -> FieldType {
        self.field_type
    }
}

#[cfg(test)]
mod tests {
    use crate::dbf::{DbfReader, DbfVersion};
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
}
