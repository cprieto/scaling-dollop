use std::fmt::{Display, Formatter};
use crate::errors;
use crate::errors::Error;

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
            _ => Err(Error::FileFormat(format!("unknown version: {value:#04x}")))
        }
    }
}

struct DbfHeader {
    version: DbfVersion,
    last_update: Date,
}
