//! Getting the chosen build onto the disk, and proving it is what it claims to be.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use super::catalog;
use super::fw::{self, FwFile};
use super::progress::{FlashProgress, FlashStage};

/// The folder holding the user's own builds.
pub fn user_firmware_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("firmware")))
        .unwrap_or_else(|| PathBuf::from("./firmware"))
}

/// Scratch space for downloads.
pub fn scratch_dir() -> PathBuf {
    std::env::temp_dir().join("steam-puck-flasher")
}

/// Put the chosen build on disk.
pub(crate) fn materialise(
    chosen: &catalog::Entry,
    report: &mut dyn FnMut(FlashProgress),
) -> Result<PathBuf> {
    match &chosen.source {
        catalog::Source::Local { path } => {
            report(FlashProgress::note(
                FlashStage::Downloading,
                format!("local build: {}", path.display()),
            ));
            Ok(path.clone())
        }
        catalog::Source::Remote { url } => {
            report(FlashProgress::note(
                FlashStage::Downloading,
                format!("downloading {url}"),
            ));
            let mut resp = ureq::get(url).call().context("downloading firmware")?;
            let bytes = resp
                .body_mut()
                .read_to_vec()
                .context("reading the response")?;

            let dir = scratch_dir();
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("creating the scratch directory {}", dir.display()))?;

            let path = dir.join(&chosen.file_name);
            let partial = dir.join(format!("{}.part", chosen.file_name));
            std::fs::write(&partial, &bytes)
                .with_context(|| format!("writing {}", partial.display()))?;
            std::fs::rename(&partial, &path)
                .with_context(|| format!("moving the download into place at {}", path.display()))?;

            report(FlashProgress::note(
                FlashStage::Downloading,
                format!(
                    "saved {} ({})",
                    path.display(),
                    fw::human_size(bytes.len() as u64)
                ),
            ));
            Ok(path)
        }
    }
}

/// Check the bytes against every hash the catalogue knows about.
pub(crate) fn verify_against_catalogue(
    chosen: &catalog::Entry,
    bytes: &[u8],
    path: &Path,
    report: &mut dyn FnMut(FlashProgress),
) -> Result<()> {
    let image = FwFile::parse(bytes.to_vec())?;
    let got_crc = crc32fast::hash(image.payload());

    if chosen.sha256.is_empty() {
        report(FlashProgress::note(
            FlashStage::Verifying,
            "container is self-consistent (header CRC32 and size match the payload); \
             no upstream hashes exist for a local build",
        ));
        return Ok(());
    }

    let want_crc = chosen.crc32;
    if got_crc != want_crc {
        bail!(
            "CRC32 mismatch for {}: the index says 0x{want_crc:08X}, the payload hashes to 0x{got_crc:08X}",
            path.display()
        );
    }

    use sha2::{Digest, Sha256};
    let got_sha = format!("{:x}", Sha256::digest(bytes));
    if got_sha != chosen.sha256 {
        bail!(
            "SHA256 mismatch for {}: the index says {}, the file hashes to {}",
            path.display(),
            chosen.sha256,
            got_sha
        );
    }

    report(FlashProgress::note(
        FlashStage::Verifying,
        "verified against the upstream index (CRC32 over the payload, SHA256 over the file)",
    ));
    Ok(())
}
