#![allow(dead_code)]

use crate::errors::Error;
use crate::errors::Error::FileFormat;
use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use strum::FromRepr;
use time::{Date, Month};

#[derive(Debug, PartialEq, FromRepr)]
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
}

struct Header {
    version: DbfVersion,
    last_update: Date,
    num_records: u32,
    record_start: u16,
    record_length: u16,
}

pub struct DbfReader<'a, R: Read + Seek> {
    reader: &'a mut R,
    header: Header,
}

impl<'a, R: Read + Seek> DbfReader<'a, R> {
    pub fn from_reader(reader: &'a mut R) -> Result<Self, Error> {
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
        })
    }
}

enum FieldType {
    // In DB3
    Character,
    Numeric,
    Date,
    Logical,
    // In DB4, FoxPro
    Float,
}

struct Field {
    name: String,
    field_type: FieldType,
}

#[cfg(test)]
mod tests {
    use time::{Date, Month};
    use crate::dbf::{DbfReader, DbfVersion};
    use crate::sample_file;

    #[test]
    fn dbase3_is_not_y2k_ready() -> anyhow::Result<()> {
        let mut reader = sample_file("db3.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;

        // dbase3 thinks is 1900!
        assert_eq!(dbf.header.last_update, Date::from_calendar_date(1926, Month::February, 16)?);

        let mut reader = sample_file("fox1.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;
        // same with FoxPro, they are broken :(
        assert_eq!(dbf.header.last_update, Date::from_calendar_date(1926, Month::February, 18)?);

        let mut reader = sample_file("db4.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;

        // dbase4 and later are ok
        assert_eq!(dbf.header.last_update, Date::from_calendar_date(2026, Month::February, 16)?);

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

        // same as FoxPro
        let mut reader = sample_file("fox1.dbf")?;
        let dbf = DbfReader::from_reader(&mut reader)?;

        assert_eq!(DbfVersion::Dbase, dbf.header.version);


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