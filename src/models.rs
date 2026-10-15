//! Display-side types for the GUI.

use crate::flasher::catalog::{self, Channel};

/// One row of the firmware catalogue.
#[derive(Debug, Clone)]
pub struct Row {
    pub file_name: String,
    pub channel: Channel,
    pub date: String,
    pub size: u64,
    pub crc32: u32,
    pub sha256: String,
}

impl Row {
    pub fn from_entry(e: &catalog::Entry) -> Self {
        Self {
            file_name: e.file_name.clone(),
            channel: e.channel,
            date: e.date.clone(),
            size: e.size,
            crc32: e.crc32,
            sha256: e.sha256.clone(),
        }
    }

    pub fn day(&self) -> &str {
        let d = self.date.as_str();
        let shaped = d.len() > 10
            && d.as_bytes()[4] == b'-'
            && d.as_bytes()[7] == b'-'
            && d.as_bytes()[..4].iter().all(u8::is_ascii_digit)
            && d.as_bytes()[5..7].iter().all(u8::is_ascii_digit)
            && d.as_bytes()[8..10].iter().all(u8::is_ascii_digit);
        if shaped {
            &d[..10]
        } else {
            "unknown"
        }
    }

    pub fn size_h(&self) -> String {
        human_size(self.size)
    }

    pub fn crc_h(&self) -> String {
        format!("0x{:08X}", self.crc32)
    }
}

fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    if bytes as f64 >= KIB {
        format!("{:.0} KiB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

/// Severity of a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
    Ok,
}

/// One line in the log pane.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub at: String,
    pub level: Level,
    pub text: String,
}

impl LogLine {
    pub fn new(level: Level, text: impl Into<String>) -> Self {
        Self {
            at: chrono::Local::now().format("%H:%M:%S").to_string(),
            level,
            text: text.into(),
        }
    }
}

/// The identity of the build.
#[derive(Debug, Clone)]
pub struct Selection {
    pub serial: String,
    pub row: Row,
}

/// Which screen we are on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Serial,
    Catalogue,
    Confirm,
    Flashing,
    Done,
}

impl Step {
    pub fn label(self) -> &'static str {
        match self {
            Step::Serial => "Serial",
            Step::Catalogue => "Catalogue",
            Step::Confirm => "Confirm",
            Step::Flashing => "Flashing",
            Step::Done => "Done",
        }
    }
}

/// How far the flash is shown by a progress bar.
#[derive(Debug, Default)]
pub struct Meter {
    pub fraction: f32,
    pub chunk: u32,
    pub total_chunks: u32,
    pub headline: String,
}

impl Meter {
    pub fn apply(&mut self, p: &crate::flasher::FlashProgress) {
        self.headline = p.message.clone();
        if let (Some(done), Some(total)) = (p.bytes_written, p.total_bytes) {
            if total > 0 {
                self.fraction = (done as f32 / total as f32).clamp(0.0, 1.0);
            }
        }
        if let (Some(c), Some(t)) = (p.chunk, p.total_chunks) {
            self.chunk = c;
            self.total_chunks = t;
        }
    }

    pub fn chunk_line(&self) -> Option<String> {
        (self.total_chunks > 0).then(|| format!("chunk {} of {}", self.chunk, self.total_chunks))
    }
}
