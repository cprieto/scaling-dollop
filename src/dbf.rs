use crate::SliceUntilTerminator;
use crate::errors::Error::{self, Fieldvalue, FileFormat, NotSupported};
use byteorder::{LittleEndian, ReadBytesExt};
use rust_decimal::Decimal;
use std::io::{Read, Seek, SeekFrom};
use std::str::FromStr;
use std::sync::Arc;
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
            fields: Arc::new(fields),
        })
    }

    /// Fields defined in this DBF table
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Returns iterator to rows in the DBF table
    /// this includes deleted rows
    /// only one iterator at a time!
    pub fn rows(&mut self) -> Rows<'_, R> {
        Rows {
            reader: &mut self.reader,
            fields: self.fields.clone(),
            record_start: self.header.record_start,
            record_size: self.header.record_length,
            current: 0,
            total: self.header.num_records,
        }
    }
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
    name: String,
    offset: u16,
    field_type: FieldType,
}

impl Field {
    fn new<R: Read + Seek>(reader: &mut R, offset: u16) -> Result<Self, Error> {
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

/// A value contained in a field for a row
#[derive(PartialEq, Debug)]
pub enum Value {
    Character(String),
    Numeric(Decimal),
    Float(f64),
    Date(time::Date),
    Logical(bool),
    Memo(String),
    Integer(i32),
    Currency(f64),
    DateTime(time::PrimitiveDateTime),
    Double(f64),
    Null,
}

/// Represent a row in a DBF file
pub struct Row {
    fields: Arc<Vec<Field>>,
    data: Vec<u8>,
}

#[inline]
fn to_text(bytes: &[u8]) -> Result<&str, Error> {
    std::str::from_utf8(bytes).map_err(|_| Fieldvalue("invalid field value for text".into()))
}

impl Row {
    /// Tell us if the deleted flag is set for this record
    pub fn is_deleted(&self) -> bool {
        self.data[0] == 0x2a
    }

    /// Gets a column by its name
    pub fn get(&self, column: &str) -> Result<Value, Error> {
        // first find field in list of fields
        let field = self
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(column))
            .ok_or(Error::FieldNotFound(column.to_string()))?;

        let start = field.offset as usize;
        let end = (field.offset + field.size()) as usize;

        let value = match field.field_type {
            FieldType::Character(_) => {
                if self.data[start..end].iter().all(|char| *char == 0x20) {
                    Value::Null
                } else {
                    let text = to_text(&self.data[start..end])?;
                    let text = text.trim_ascii_end();
                    Value::Character(text.to_owned())
                }
            }
            FieldType::Numeric { decimal, .. } => {
                if self.data[start..end].iter().all(|char| *char == 0x20) {
                    Value::Null
                } else {
                    let text = to_text(&self.data[start..end])?;
                    let text = text.trim_ascii_start();

                    let mut number = Decimal::from_str(text)
                        .map_err(|_| Fieldvalue(format!("invalid decimal value: {text}")))?;
                    number.rescale(decimal as u32);

                    Value::Numeric(number)
                }
            }
            FieldType::Logical => {
                let byte = &self.data[start];
                match byte {
                    0x46 | 0x4e => Value::Logical(false),
                    0x54 | 0x59 => Value::Logical(true),
                    0x3f => Value::Null,
                    _ => return Err(Fieldvalue(format!("invalid logical: 0x{byte}"))),
                }
            }
            FieldType::Date => {
                if self.data[start..end].iter().all(|char| *char == 0x20) {
                    Value::Null
                } else {
                    let text = to_text(&self.data[start..end])?;

                    let year = text[0..4]
                        .parse::<i32>()
                        .map_err(|_| Fieldvalue(format!("invalid date {}", &text[0..4])))?;
                    let month = text[4..6]
                        .parse::<u8>()
                        .ok()
                        .and_then(|month| Month::try_from(month).ok())
                        .ok_or(Fieldvalue(format!("invalid date {}", &text[4..6])))?;
                    let day = text[6..8]
                        .parse::<u8>()
                        .map_err(|_| Fieldvalue(format!("invalid date: {}", &text[6..8])))?;
                    let date = Date::from_calendar_date(year, month, day)
                        .map_err(|_| Fieldvalue(format!("invalid date: {text}")))?;

                    Value::Date(date)
                }
            }
            FieldType::Memo => return Err(NotSupported),
            // DBF4...
            FieldType::Float { .. } => todo!(),
            // VFP
            FieldType::Integer => todo!(),
            FieldType::Currency => todo!(),
            FieldType::DateTime => todo!(),
            FieldType::Double { .. } => todo!(),
        };

        Ok(value)
    }

    /// Returns fields in this row
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}

pub struct Rows<'a, R: Read + Seek> {
    reader: &'a mut R,
    record_size: u16,
    record_start: u16,
    fields: Arc<Vec<Field>>,
    current: u32,
    total: u32,
}

impl<'a, R: Read + Seek> Iterator for Rows<'a, R> {
    type Item = Result<Row, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }
        let position = (self.record_start as u32) + (self.record_size as u32) * self.current;
        if let Err(err) = self.reader.seek(SeekFrom::Start(position as u64)) {
            return Some(Err(err.into()));
        }

        let mut data = vec![0u8; self.record_size as usize];
        if let Err(err) = self.reader.read_exact(&mut data) {
            return Some(Err(err.into()));
        }

        self.current += 1;

        let row = Row {
            fields: self.fields.clone(),
            data,
        };

        Some(Ok(row))
    }
}

#[cfg(test)]
mod tests {
    use crate::dbf::{DbfReader, DbfVersion, Field, FieldType, Row, Value};
    use crate::sample_file;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use std::sync::Arc;
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
    fn parsing_row_data() -> anyhow::Result<()> {
        let fields = vec![
            Field {
                name: "NAME".to_string(),
                offset: 1,
                field_type: FieldType::Character(20),
            },
            Field {
                name: "PRICE".to_string(),
                offset: 21,
                field_type: FieldType::Numeric {
                    size: 10,
                    decimal: 2,
                },
            },
            Field {
                name: "QTY".to_string(),
                offset: 31,
                field_type: FieldType::Numeric {
                    size: 6,
                    decimal: 0,
                },
            },
            Field {
                name: "ACTIVE".to_string(),
                offset: 37,
                field_type: FieldType::Logical,
            },
            Field {
                name: "ADDED".to_string(),
                offset: 38,
                field_type: FieldType::Date,
            },
        ];
        let data: [u8; 0x2E] = [
            0x20, 0x57, 0x69, 0x64, 0x67, 0x65, 0x74, 0x20, 0x50, 0x72, 0x6F, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x32, 0x39,
            0x2E, 0x39, 0x39, 0x20, 0x20, 0x20, 0x31, 0x35, 0x30, 0x54, 0x31, 0x39, 0x32, 0x35,
            0x30, 0x31, 0x31, 0x35,
        ];

        let row = Row {
            fields: Arc::new(fields),
            data: data.to_vec(),
        };

        assert_eq!(5, row.fields.len());

        // Getting a date works
        let date = row.get("ADDED")?;
        assert!(
            matches!(date, Value::Date(d) if d == Date::from_calendar_date(1925, Month::January, 15)?)
        );

        let active = row.get("ACTIVE")?;
        assert!(matches!(active, Value::Logical(l) if l));

        let name = row.get("NAME")?;
        assert!(matches!(name, Value::Character(s) if s == "Widget Pro"));

        let price = row.get("PRICE")?;
        assert!(matches!(price, Value::Numeric(n) if n == Decimal::from_str("29.99")?));

        let qty = row.get("QTY")?;
        if let Value::Numeric(n) = qty {
            assert_eq!(n.mantissa(), 150);
            assert_eq!(n.scale(), 0);
        } else {
            panic!("qty should be numeric")
        }

        // check is not deleted
        assert!(!row.is_deleted());

        // uppercase or lowercase is ok
        let name = row.get("name")?;
        assert!(matches!(name, Value::Character(s) if s == "Widget Pro"));

        Ok(())
    }

    #[test]
    fn read_rows() -> anyhow::Result<()> {
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
