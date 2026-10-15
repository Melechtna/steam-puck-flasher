//! The 128-byte UICR provisioning blob.

use anyhow::{bail, Result};

use super::fw::crc32;

/// The provisioning magic the application requires in order to come up with its
/// full USB descriptor set (charging, serial number, status interface).
pub const PROVISIONING_MAGIC: u32 = 0xAC32_A429;

/// Hardware id for the puck.
pub const PUCK_HARDWARE_ID: u32 = 71;

/// Layout of the 128-byte provisioning blob.
pub const PROVISIONING_LEN: usize = 128;
const MAGIC_OFFSET: usize = 0;
const HARDWARE_ID_OFFSET: usize = 4;
const SERIAL_OFFSET: usize = 8;
const SERIAL_MAX: usize = 20;
const CRC_OFFSET: usize = 124;

/// Build the 128-byte UICR provisioning blob for a serial number.
pub fn build_provisioning(serial: &str) -> Result<[u8; PROVISIONING_LEN]> {
    let bytes = serial.as_bytes();
    if bytes.is_empty() || bytes.len() > SERIAL_MAX {
        bail!("serial must be 1..={SERIAL_MAX} characters");
    }

    let mut blob = [0u8; PROVISIONING_LEN];
    blob[MAGIC_OFFSET..MAGIC_OFFSET + 4].copy_from_slice(&PROVISIONING_MAGIC.to_le_bytes());
    blob[HARDWARE_ID_OFFSET..HARDWARE_ID_OFFSET + 4]
        .copy_from_slice(&PUCK_HARDWARE_ID.to_le_bytes());
    blob[SERIAL_OFFSET..SERIAL_OFFSET + bytes.len()].copy_from_slice(bytes);

    let crc = crc32(&blob[..CRC_OFFSET]);
    blob[CRC_OFFSET..].copy_from_slice(&crc.to_le_bytes());
    Ok(blob)
}

/// A provisioning blob recovered from the device.
pub struct Provisioning {
    blob: [u8; PROVISIONING_LEN],
}

impl Provisioning {
    pub fn hardware_id(&self) -> u32 {
        let at = HARDWARE_ID_OFFSET;
        u32::from_le_bytes(self.blob[at..at + 4].try_into().expect("fixed layout"))
    }

    pub fn serial(&self) -> String {
        String::from_utf8_lossy(&self.blob[SERIAL_OFFSET..CRC_OFFSET])
            .trim_end_matches('\0')
            .to_string()
    }

    pub fn crc32(&self) -> u32 {
        let at = CRC_OFFSET;
        u32::from_le_bytes(self.blob[at..at + 4].try_into().expect("fixed layout"))
    }

    pub fn is_identical_to(&self, other: &[u8; PROVISIONING_LEN]) -> bool {
        &self.blob == other
    }
}

/// Find and decode the provisioning blob.
pub fn locate_provisioning(reply: &[u8]) -> Option<Provisioning> {
    let needle = PROVISIONING_MAGIC.to_le_bytes();

    if reply.len() < 4 {
        return None;
    }
    for start in 0..=(reply.len() - 4) {
        if reply[start..start + 4] != needle {
            continue;
        }
        let Some(end) = start.checked_add(PROVISIONING_LEN) else {
            continue;
        };
        if reply.len() < end {
            continue;
        }
        let blob: [u8; PROVISIONING_LEN] = reply[start..end].try_into().ok()?;

        let at = CRC_OFFSET;
        let stored = u32::from_le_bytes(blob[at..at + 4].try_into().expect("fixed layout"));
        if stored != crc32(&blob[..CRC_OFFSET]) {
            continue;
        }

        return Some(Provisioning { blob });
    }
    None
}
