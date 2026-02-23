use byteorder::ReadBytesExt;
use std::io::{ErrorKind, Read, Result as IOResult};

pub mod dbf;
pub mod errors;
pub mod memo;

pub trait SliceUntilTerminator<'a> {
    fn until_terminator(&'a self, delimiter: &[u8]) -> &'a [u8];
}

impl<'a> SliceUntilTerminator<'a> for [u8] {
    fn until_terminator(&'a self, delimiter: &[u8]) -> &'a [u8] {
        self.windows(delimiter.len())
            .position(|w| w == delimiter)
            .map_or(self, |pos| &self[..pos])
    }
}

trait ReaderUntilTerminator {
    fn read_until_terminator(&mut self, delimiter: &[u8]) -> IOResult<Vec<u8>>;
}

impl<R> ReaderUntilTerminator for R
where
    R: Read,
{
    fn read_until_terminator(&mut self, delimiter: &[u8]) -> IOResult<Vec<u8>> {
        assert!(!delimiter.is_empty(), "delimiter must not be empty");

        let mut output = vec![];
        loop {
            match self.read_u8() {
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
}

#[cfg(test)]
fn sample_file(name: &str) -> std::io::Result<std::fs::File> {
    use std::fs::File;

    let path = format!("{}/samples/{name}", env!("CARGO_MANIFEST_DIR"));
    File::open(path)
}
