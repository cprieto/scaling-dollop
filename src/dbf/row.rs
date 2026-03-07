use crate::dbf::header::{Field, FieldType};
use crate::errors::Error;
use crate::errors::Error::{Fieldvalue, NotSupported};
use byteorder::{LittleEndian, ReadBytesExt};
use rust_decimal::Decimal;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::str::FromStr;
use std::sync::Arc;
use time::{Date, Month, PrimitiveDateTime, Time};

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
    Currency(Decimal),
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
            .find(|field| field.name().eq_ignore_ascii_case(column))
            .ok_or(Error::FieldNotFound(column.to_string()))?;

        let start = field.offset as usize;
        let end = (field.offset + field.size()) as usize;

        let value = match field.field_type() {
            FieldType::Character(_) => {
                if self.data[start..end].iter().all(|char| *char == 0x20) {
                    return Ok(Value::Null);
                }
                let text = to_text(&self.data[start..end])?;
                let text = text.trim_ascii_end();
                Value::Character(text.to_owned())
            }
            FieldType::Numeric { decimal, .. } => {
                if self.data[start..end].iter().all(|char| *char == 0x20) {
                    return Ok(Value::Null);
                }
                let text = to_text(&self.data[start..end])?;
                let text = text.trim_ascii_start();

                let mut number = Decimal::from_str(text)
                    .map_err(|_| Fieldvalue(format!("invalid decimal value: {text}")))?;
                number.rescale(decimal as u32);

                Value::Numeric(number)
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
                    return Ok(Value::Null);
                }
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
            FieldType::Memo => return Err(NotSupported),
            // DBF4...
            FieldType::Float { .. } => {
                if self.data[start..end].iter().all(|char| *char == 0x20) {
                    return Ok(Value::Null);
                }

                let text = to_text(&self.data[start..end])?;
                let text = text.trim_ascii_start();
                let value = text
                    .parse::<f64>()
                    .map_err(|_| Fieldvalue(format!("invalid float: {text}")))?;

                Value::Float(value)
            }
            // VFP
            FieldType::Integer => {
                let mut cursor = Cursor::new(&self.data[start..end]);
                let value = cursor.read_i32::<LittleEndian>()?;

                Value::Integer(value)
            }
            FieldType::Currency => {
                let mut cursor = Cursor::new(&self.data[start..end]);

                let value = cursor.read_i64::<LittleEndian>()?;
                let value = Decimal::new(value, 4);

                Value::Currency(value)
            }
            FieldType::DateTime => {
                let mut cursor = Cursor::new(&self.data[start..end]);
                let days = cursor.read_u32::<LittleEndian>()?;
                let millis = cursor.read_u32::<LittleEndian>()?;

                if days == 0 && millis == 0 {
                    return Ok(Value::Null);
                }

                let date = Date::from_julian_day(days as i32)
                    .map_err(|_| Fieldvalue(format!("invalid days in gregorian: {days}")))?;

                let hour = (millis / 3_600_000) as u8;
                let min = ((millis % 3_600_000) / 60_000) as u8;
                let sec = ((millis % 60_000) / 1_000) as u8;
                let ms = (millis % 1_000) as u16;

                let time = Time::from_hms_milli(hour, min, sec, ms)
                    .map_err(|_| Fieldvalue(format!("invalid time: {hour}:{min}:{sec}:{ms}")))?;

                Value::DateTime(PrimitiveDateTime::new(date, time))
            }
            FieldType::Double { .. } => {
                let mut cursor = Cursor::new(&self.data[start..end]);
                let value = cursor.read_f64::<LittleEndian>()?;

                Value::Double(value)
            }
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

impl<'a, R: Read + Seek> Rows<'a, R> {
    pub(crate) fn new(
        reader: &'a mut R,
        record_size: u16,
        record_start: u16,
        total: u32,
        fields: Arc<Vec<Field>>,
    ) -> Self {
        Self {
            reader,
            record_size,
            record_start,
            fields,
            total,
            current: 0,
        }
    }
}

impl<'a, R: Read + Seek> Iterator for Rows<'a, R> {
    type Item = Result<Row, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }
        let position =
            (self.record_start as u64) + (self.record_size as u64) * (self.current as u64);
        self.current += 1;
        if let Err(err) = self.reader.seek(SeekFrom::Start(position as u64)) {
            return Some(Err(err.into()));
        }

        let mut data = vec![0u8; self.record_size as usize];
        if let Err(err) = self.reader.read_exact(&mut data) {
            return Some(Err(err.into()));
        }

        let row = Row {
            fields: Arc::clone(&self.fields),
            data,
        };

        Some(Ok(row))
    }
}

#[cfg(test)]
mod tests {
    use crate::dbf::header::{Field, FieldType};
    use crate::dbf::row::{Row, Value};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use std::str::FromStr;
    use std::sync::Arc;
    use time::{Date, Month, PrimitiveDateTime, Time};

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
    fn read_rows_vfp() -> anyhow::Result<()> {
        let data: [u8; 0x55] = [
            0x20, 0x01, 0x00, 0x00, 0x00, 0x57, 0x69, 0x64, 0x67, 0x65, 0x74, 0x20, 0x50, 0x72,
            0x6F, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x32, 0x39, 0x2E, 0x39, 0x39, 0x78, 0x5D, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x20, 0x20, 0x20, 0x31, 0x35, 0x30, 0x20, 0x20, 0x20, 0x20, 0x30, 0x2E, 0x33,
            0x35, 0x30, 0x30, 0x03, 0x09, 0x8A, 0x1F, 0x63, 0xEE, 0xDE, 0x3F, 0x54, 0x31, 0x39,
            0x32, 0x35, 0x30, 0x31, 0x31, 0x35, 0x66, 0xFD, 0x24, 0x00, 0x40, 0xC8, 0x40, 0x02,
            0x20,
        ];

        let fields = vec![
            Field {
                name: "ID".to_string(),
                offset: 1,
                field_type: FieldType::Integer,
            },
            Field {
                name: "NAME".to_string(),
                offset: 5,
                field_type: FieldType::Character(20),
            },
            Field {
                name: "PRICE".to_string(),
                offset: 25,
                field_type: FieldType::Numeric {
                    size: 10,
                    decimal: 2,
                },
            },
            Field {
                name: "COST".to_string(),
                offset: 35,
                field_type: FieldType::Currency,
            },
            Field {
                name: "QTY".to_string(),
                offset: 43,
                field_type: FieldType::Numeric {
                    size: 6,
                    decimal: 0,
                },
            },
            Field {
                name: "WEIGHT".to_string(),
                offset: 49,
                field_type: FieldType::Numeric {
                    size: 6,
                    decimal: 0,
                },
            },
            Field {
                name: "MARGIN".to_string(),
                offset: 59,
                field_type: FieldType::Double { decimal: 4 },
            },
            Field {
                name: "ACTIVE".to_string(),
                offset: 67,
                field_type: FieldType::Logical,
            },
            Field {
                name: "ADDED".to_string(),
                offset: 68,
                field_type: FieldType::Date,
            },
            Field {
                name: "UPDATED".to_string(),
                offset: 76,
                field_type: FieldType::DateTime,
            },
        ];

        let row = Row {
            fields: Arc::new(fields),
            data: data.to_vec(),
        };

        assert_eq!(10, row.fields.len());

        assert_eq!(Value::Integer(1), row.get("ID")?);

        assert_eq!(
            Value::Currency(Decimal::from_f64(15.5000).unwrap()),
            row.get("COST")?
        );

        assert_eq!(Value::Double(0.4833), row.get("MARGIN")?);

        // VFP is fucked up with dates as well
        let date = Date::from_calendar_date(1925, Month::January, 15)?;
        let time = Time::from_hms(10, 30, 0)?;

        let date_time = PrimitiveDateTime::new(date, time);

        assert_eq!(Value::DateTime(date_time), row.get("UPDATED")?);

        Ok(())
    }
}
