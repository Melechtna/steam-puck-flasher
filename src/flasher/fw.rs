//! The `.fw` container.

use anyhow::{bail, Result};
use crc32fast::Hasher as Crc32;
use sha2::{Digest, Sha256};
use std::time::SystemTime;

pub const HEADER_LEN: usize = 32;

/// Image magic for the puck.
pub const IMAGE_MAGIC: u32 = 0x2E79_5631;

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut h = Crc32::new();
    h.update(bytes);
    h.finalize()
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// A parsed, validated .fw file.
#[derive(Debug, Clone)]
pub struct FwFile {
    bytes: Vec<u8>,
    pub magic: u32,
    pub payload_size: u32,
    pub payload_crc32: u32,
}

impl FwFile {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            bail!(
                "file is {} bytes, too short to hold a 32-byte header",
                bytes.len()
            );
        }

        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let payload_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let payload_crc32 = u32::from_le_bytes(bytes[8..12].try_into().unwrap());

        let actual_payload_len = (bytes.len() - HEADER_LEN) as u32;
        if payload_size != actual_payload_len {
            bail!(
                "header claims a {payload_size}-byte payload but the file carries {actual_payload_len} bytes \
                 -- truncated download?"
            );
        }

        let actual_crc = crc32(&bytes[HEADER_LEN..]);
        if actual_crc != payload_crc32 {
            bail!(
                "payload CRC32 is 0x{actual_crc:08X} but the header says 0x{payload_crc32:08X} \
                 -- corrupt download?"
            );
        }

        Ok(Self {
            bytes,
            magic,
            payload_size,
            payload_crc32,
        })
    }

    pub fn payload(&self) -> &[u8] {
        &self.bytes[HEADER_LEN..]
    }
}

/// Build the 32-byte container header for a given firmware.
pub fn build_header(payload: &[u8]) -> [u8; HEADER_LEN] {
    let crc = crc32fast::hash(payload);
    let mut header = [0u8; HEADER_LEN];
    header[0..4].copy_from_slice(&IMAGE_MAGIC.to_le_bytes());
    header[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    header[8..12].copy_from_slice(&crc.to_le_bytes());
    header
}

/// Render bytes as a hex/ASCII dump, for showing a device reply verbatim.
pub fn hex_dump(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (row, chunk) in bytes.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        out.push_str(&format!("    {row:3} 0x{:04x}  ", row * 16));
        out.push_str(&format!("{:<47}  |{ascii}|\n", hex.join(" ")));
    }
    out
}

/// Format a `SystemTime` as a local string for display only.
pub fn format_time(t: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Human-readable byte count for progress lines.
pub fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    if bytes as f64 >= KIB {
        format!("{:.0} KiB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}
