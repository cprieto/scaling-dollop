use crate::SliceUntilTerminator;
use crate::errors::Error;
use crate::errors::Error::FileFormat;
use byteorder::ReadBytesExt;
use std::io::{Read, Seek, SeekFrom};
use strum::{Display, FromRepr};
use time::Date;

#[derive(Debug, PartialEq, FromRepr, Display)]
#[repr(u8)]
pub(crate) enum DbfVersion {
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
pub(crate) struct Header {
    pub(crate) version: DbfVersion,
    pub(crate) last_update: Date,
    pub(crate) num_records: u32,
    pub(crate) record_start: u16,
    pub(crate) record_length: u16,
}

/// The field (column) type and its constraints
#[derive(Clone, Copy, Debug)]
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
    Double { decimal: u8 },
}

/// A field (column) defined in a given DBF table
pub struct Field {
    pub(crate) name: String,
    pub(crate) offset: u16,
    pub(crate) field_type: FieldType,
}

impl Field {
    pub(crate) fn new<R: Read + Seek>(reader: &mut R, offset: u16) -> Result<Self, Error> {
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
            what @ (0x4e | 0x42 | 0x46) => {
                // Read length and decimal places
                let size = reader.read_u8()?;
                let decimal = reader.read_u8()?;
                if what == 0x4e {
                    FieldType::Numeric { size, decimal }
                } else if what == 0x42 {
                    FieldType::Double { decimal }
                } else {
                    FieldType::Float { size, decimal }
                }
            }
            0x44 => FieldType::Date,
            0x4c => FieldType::Logical,
            0x4d => FieldType::Memo,
            0x49 => FieldType::Integer,
            0x59 => FieldType::Currency,
            0x54 => FieldType::DateTime,
            _ => return Err(FileFormat(format!("invalid field type: {field_type}"))),
        };

        // While Field info is 32 bytes, we don't have much
        // to read for now, it has more info like autoincrement,
        // indices, etc...

        Ok(Self {
            name: name.into_owned(),
            offset,
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

    pub fn size(&self) -> u16 {
        match self.field_type {
            FieldType::Character(size) => size as u16,
            FieldType::Numeric { size, .. } => size as u16,
            FieldType::Float { size, .. } => size as u16,
            FieldType::Date => 8,
            FieldType::Logical => 1,
            FieldType::Memo => 10,
            FieldType::Integer => 4,
            FieldType::Double { .. } => 8,
            FieldType::Currency => 8,
            FieldType::DateTime => 8,
        }
    }
}
