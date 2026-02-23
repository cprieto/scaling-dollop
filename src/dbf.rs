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

#[expect(dead_code)]
struct Header {
    version: DbfVersion,
    last_update: Date,
    num_records: u32,
    record_start: u16,
    record_length: u16,
}

#[expect(dead_code)]
pub struct DbfReader<R: Read + Seek> {
    reader: R,
    header: Header,
    fields: Vec<Field>,
}

impl<R: Read + Seek> DbfReader<R> {
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

        // Number of fields
        let num_fields = ((record_start - 1) / 32 - 1) as u64;

        const FIELD_START: u64 = 32;
        let fields = (0..num_fields)
            .map(|loc| {
                reader.seek(SeekFrom::Start(FIELD_START + loc * 32))?;
                Field::from_reader(&mut reader)
            })
            .collect::<Result<Vec<_>, _>>()?;

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
}

#[derive(Debug, PartialEq, FromRepr, SDisplay)]
#[repr(u8)]
enum FieldType {
    // In DBase 3
    Character = 0x43,
    Numeric = 0x4e,
    Date = 0x44,
    Logical = 0x4c,
    Memo = 0x4d,
    // DBase 4
    Float = 0x46,
    // Visual FoxPro
    Integer = 0x49,
    Currency = 0x59,
    Double = 0x42,
    DateTime = 0x54,
}

#[expect(dead_code)]
struct Field {
    name: String,
    field_type: FieldType,
    length: u8,
    decimal: u8,
}

impl Field {
    fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut name = [0u8; 11];
        reader.read_exact(&mut name)?;
        let name = name.until_terminator(&[0]);
        let name = String::from_utf8_lossy(name);

        let field_type = reader.read_u8()?;
        let field_type = FieldType::from_repr(field_type)
            .ok_or(FileFormat(format!("invalid field type: {field_type}")))?;

        reader.seek(SeekFrom::Current(4))?;
        let length = reader.read_u8()?;

        let decimal = reader.read_u8()?;

        Ok(Self {
            name: name.into_owned(),
            field_type,
            length,
            decimal,
        })
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

        // let mut reader = sample_file("vfp.dbf")?;
        // let dbf = DbfReader::from_reader(&mut reader)?;
        // assert_eq!(
        //     dbf.header.last_update,
        //     Date::from_calendar_date(1926, Month::February, 23)?
        // );

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
    #[ignore = "Fix Visual FoxPro parsing"]
    fn vfp_is_its_own_type() -> anyhow::Result<()> {
        let mut reader = sample_file("vfp.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        assert_eq!(DbfVersion::VisualFoxPro, dbf.header.version);

        Ok(())
    }

    #[test]
    #[ignore = "Fix Visual FoxPro parsing"]
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
}
