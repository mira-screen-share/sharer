use crate::OutputSink;
use crate::Result;
use async_trait::async_trait;
use bytes::Bytes;
use std::fs::File;
use std::io::Write;
use std::time::Duration;

pub struct FileOutput {
    file: File,
    file_audio: File,
}

impl FileOutput {
    pub fn new(path: &str) -> Self {
        Self {
            file: File::create(path).unwrap(),
            file_audio: File::create(path.to_string() + ".opus").unwrap(),
        }
    }
}

#[async_trait]
impl OutputSink for FileOutput {
    async fn write(&mut self, input: Bytes) -> Result<()> {
        self.file.write_all(&input)?;
        Ok(())
    }
    async fn write_audio(&mut self, input: Bytes, _duration: Duration) -> Result<()> {
        self.file_audio.write_all(&input)?;
        Ok(())
    }
}

impl Drop for FileOutput {
    fn drop(&mut self) {
        self.file.flush().unwrap();
    }
}
