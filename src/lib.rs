use byteorder::ReadBytesExt;
use std::io::{ErrorKind, Read, Result as IOResult};

pub mod dbf;
pub mod errors;
pub mod memo;

fn read_until_terminator<R: Read>(reader: &mut R, delimiter: &[u8]) -> IOResult<Vec<u8>> {
    assert!(!delimiter.is_empty(), "delimiter must not be empty");

    let mut output = vec![];
    loop {
        match reader.read_u8() {
            Ok(b) => {
                output.push(b);
                if output.len() >= delimiter.len() {
                    let size = output.len() - delimiter.len();
                    if output[size..] == *delimiter {
                        output.truncate(size);
                        return Ok(output);
                    }
                }
            }
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(output),
            Err(err) => return Err(err),
        }
    }
}
