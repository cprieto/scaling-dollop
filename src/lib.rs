use byteorder::ReadBytesExt;
use std::io::{ErrorKind, Read, Result as IOResult};

pub mod dbf;
pub mod errors;
pub mod memo;

pub fn slice_until_terminator(input: &[u8], delimiter: &[u8]) -> Vec<u8> {
    let mut output = vec![];
    for &b in input {
        output.push(b);
        if output.len() < delimiter.len() {
            continue;
        }

        let size = output.len() - delimiter.len();
        if output[size..] == *delimiter {
            output.truncate(size);
            return output;
        }
    }
    output
}

pub fn reader_until_terminator<R: Read>(reader: &mut R, delimiter: &[u8]) -> IOResult<Vec<u8>> {
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

#[cfg(test)]
fn sample_file(name: &str) -> std::io::Result<std::fs::File> {
    use std::fs::File;

    let path = format!("{}/samples/{name}", env!("CARGO_MANIFEST_DIR"));
    File::open(path)
}
