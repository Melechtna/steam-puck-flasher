//! Progress reporting, shared by the GUI and the CLI.

/// Where a flash has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashStage {
    ReadingCatalogue,
    Downloading,
    Verifying,
    Parsing,
    Provisioning,
    Flashing,
    Resetting,
    Done,
}

/// One report from a long-running operation.
#[derive(Debug, Clone)]
pub struct FlashProgress {
    pub stage: FlashStage,
    pub chunk: Option<u32>,
    pub total_chunks: Option<u32>,
    pub bytes_written: Option<u64>,
    pub total_bytes: Option<u64>,
    pub message: String,
}

impl FlashProgress {
    pub fn note(stage: FlashStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            chunk: None,
            total_chunks: None,
            bytes_written: None,
            total_bytes: None,
            message: message.into(),
        }
    }

    pub fn fraction(&self) -> Option<f32> {
        match (self.bytes_written, self.total_bytes) {
            (Some(done), Some(total)) if total > 0 => {
                Some((done as f32 / total as f32).clamp(0.0, 1.0))
            }
            _ => None,
        }
    }
}

/// Implemented by each frontend, the GUI logs to a pane, the CLI to stdout.
pub trait ProgressReporter {
    fn report(&mut self, p: FlashProgress);
}

/// A completed flash.
#[derive(Debug, Clone)]
pub struct FlashResult {
    pub serial: String,
    pub firmware_name: String,
    pub channel: crate::flasher::catalog::Channel,
    pub payload_size: u64,
}

/// Swallows every report, for the callers that want none.
pub struct NopReporter;

impl ProgressReporter for NopReporter {
    fn report(&mut self, _: FlashProgress) {}
}
