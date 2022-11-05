use crate::OutputSink;
use crate::Result;
use async_trait::async_trait;
use std::fs::File;
use std::io::Write;

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

#[async_trait]
impl OutputSink for FileOutput {
    async fn write(&mut self, input: &[u8]) -> Result<()> {
        self.file.write_all(input)?;
        Ok(())
    }
}

impl Drop for FileOutput {
    fn drop(&mut self) {
        self.file.flush().unwrap();
    }
}
