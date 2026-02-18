use crate::errors;
use crate::errors::Error;
use crate::errors::Error::FileFormat;
use byteorder::ReadBytesExt;
use std::fmt::{Display, Formatter};
use std::io::{Read, Seek, SeekFrom};
use time::{Date, Month};

#[derive(Debug)]
enum DbfVersion {
    Dbase,
    Dbase3WithMemo,
    Dbase4WithMemo,
}

impl Display for DbfVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DbfVersion::Dbase => write!(f, "DBase without memo"),
            DbfVersion::Dbase3WithMemo => write!(f, "DBase 3 with memo field"),
            DbfVersion::Dbase4WithMemo => write!(f, "DBase 4,5 with memo field"),
        }
    }
}

impl TryFrom<u8> for DbfVersion {
    type Error = errors::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x03 => Ok(DbfVersion::Dbase),
            0x83 => Ok(DbfVersion::Dbase3WithMemo),
            0x8B => Ok(DbfVersion::Dbase4WithMemo),
            _ => Err(Error::FileFormat(format!("unknown version: {value:#04x}"))),
        }
    }
}

struct DbfHeader {
    version: DbfVersion,
    last_update: Date,
    num_records: u32,
    start: u16,
    record_length: u16,
}

impl DbfHeader {
    fn from_reader<T: Read + Seek>(reader: &mut T) -> Result<Self, Error> {
        reader.seek(SeekFrom::Start(0))?;
        let version = reader.read_u8()?;
        let version = DbfVersion::try_from(version)?;
        let mut last_update = Vec::with_capacity(6);
        reader.take(6).read_to_end(&mut last_update)?;
        let last_update = parse_dbf_date(&last_update)?;

        todo!()
    }
}

// TODO: this implementation is fishy, I have to check against files
fn parse_dbf_date(s: &[u8]) -> Result<Date, Error> {
    let date =
        std::str::from_utf8(s).map_err(|_| FileFormat("invalid binary date in header".into()))?;

    let year = &date[0..4];
    let year = year
        .parse::<i32>()
        .map_err(|_| FileFormat(format!("invalid year for date {year}")))?;

    let month = &date[4..6];
    let month = month
        .parse::<u8>()
        .map_err(|_| FileFormat(format!("invalid month for date: {month}")))?;

    let month = Month::try_from(month)
        .map_err(|_| FileFormat(format!("invalid month for date: {month}")))?;

    let day = &date[6..8];
    let day: u8 = date
        .parse()
        .map_err(|_| FileFormat(format!("invalid day for date: {day}")))?;

    Date::from_calendar_date(year, month, day)
        .map_err(|err| FileFormat(format!("invalid date: {date}")))
}
