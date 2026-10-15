//! The image transfer itself.

use anyhow::{Context, Result};

use super::bootloader::{is_silence, trace, Bootloader, Progress, CHUNK_SIZE};

const MSG_FW_BEGIN: u16 = 0x1234;
const MSG_FW_DATA: u16 = 0x1235;
const MSG_FW_END: u16 = 0x1236;

/// Application link base.
const FLASH_BASE: u32 = 0xA000;

const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

impl Bootloader {
    pub fn flash_with(
        &mut self,
        payload: &[u8],
        header: &[u8; 32],
        mut on_progress: impl FnMut(Progress),
    ) -> Result<()> {
        if let Some(note) = self.begin(payload.len())? {
            on_progress(Progress::saying(note));
        }
        self.send_payload(payload, &mut on_progress)?;
        self.end(header)
    }
    fn begin(&mut self, len: usize) -> Result<Option<String>> {
        let mut msg = Vec::with_capacity(10);
        msg.extend_from_slice(&MSG_FW_BEGIN.to_le_bytes());
        msg.extend_from_slice(&FLASH_BASE.to_le_bytes());
        msg.extend_from_slice(&(len as u32).to_le_bytes());

        match self.request(&msg, REPLY_TIMEOUT) {
            Ok(_) => Ok(None),
            Err(e) if is_silence(&e) => {
                trace("no reply to FW_BEGIN; assuming a transfer is already open");
                Ok(Some(
                    "the bootloader already had a flash transfer open, probably from an \
                     interrupted run. Continuing into it; the whole image will be overwritten."
                        .into(),
                ))
            }
            Err(e) => Err(e).context("FW_BEGIN rejected the image"),
        }
    }

    fn send_payload(
        &mut self,
        payload: &[u8],
        on_progress: &mut impl FnMut(Progress),
    ) -> Result<()> {
        let total = payload.len().div_ceil(CHUNK_SIZE);

        for (i, chunk) in payload.chunks(CHUNK_SIZE).enumerate() {
            let mut msg = Vec::with_capacity(4 + chunk.len());
            msg.extend_from_slice(&MSG_FW_DATA.to_le_bytes());
            msg.extend_from_slice(&(chunk.len() as u16).to_le_bytes());
            msg.extend_from_slice(chunk);
            self.request(&msg, REPLY_TIMEOUT)
                .with_context(|| format!("FW_DATA chunk {} of {total}", i + 1))?;

            on_progress(Progress {
                chunk: i + 1,
                chunks: total,
                bytes: ((i + 1) * CHUNK_SIZE).min(payload.len()),
                total_bytes: payload.len(),
                note: None,
            });
        }
        Ok(())
    }

    fn end(&mut self, header: &[u8; 32]) -> Result<()> {
        let mut msg = Vec::with_capacity(4 + 32);
        msg.extend_from_slice(&MSG_FW_END.to_le_bytes());
        msg.extend_from_slice(&header[..]);
        self.request(&msg, REPLY_TIMEOUT)
            .context("FW_END rejected the image")?;
        Ok(())
    }
}
