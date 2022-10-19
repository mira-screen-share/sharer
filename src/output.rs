use std::fs::File;
use std::io::Write;
use crate::Result;

pub trait OutputSink {
    fn write(&mut self, input: &[u8]) -> Result<()>;
}

pub struct FileOutput {
    file: File,
}

impl FileOutput {
    pub fn new(path: &str) -> Self {
        Self {
            file: File::create(path).unwrap(),
        }
    }
}

impl OutputSink for FileOutput {
    fn write(&mut self, input: &[u8]) -> Result<()> {
        self.file.write_all(input)?;
        Ok(())
    }
}

impl Drop for FileOutput {
    fn drop(&mut self) {
        self.file.flush().unwrap();
    }
}
