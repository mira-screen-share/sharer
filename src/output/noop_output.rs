use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;

use crate::OutputSink;
use crate::Result;

/// Voids outputs, for testing.
pub struct NoOpOutput;

impl NoOpOutput {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl OutputSink for NoOpOutput {
    async fn write(&mut self, _input: Bytes) -> Result<()> {
        Ok(())
    }

    async fn write_audio(&mut self, _input: Bytes, _duration: Duration) -> Result<()> {
        Ok(())
    }
}
