//! Flash orchestration, catalogue, fetch-and-verify, then provision and flash.

pub mod bootloader;
pub mod catalog;
pub(crate) mod fetch;
pub mod fw;
pub mod hdlc;
pub mod progress;
pub mod provision;
pub mod reboot;
mod transfer;

use anyhow::{bail, Context, Result};
use std::path::Path;

pub use fetch::user_firmware_dir;
pub use progress::{FlashProgress, FlashResult, FlashStage, NopReporter, ProgressReporter};

use fetch::{materialise, verify_against_catalogue};
use fw::FwFile;

/// Configuration for flash.
pub struct FlashConfig<'a> {
    pub serial: String,
    pub entry: CatalogEntry,
    pub progress: Option<&'a mut dyn ProgressReporter>,
}

impl std::fmt::Debug for FlashConfig<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlashConfig")
            .field("serial", &self.serial)
            .field("entry", &self.entry)
            .field(
                "progress",
                &self.progress.as_ref().map(|_| "Some(ProgressReporter)"),
            )
            .finish()
    }
}

/// A firmware catalogue entry.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub file_name: String,
    pub channel: catalog::Channel,
    pub size: u64,
    pub first_seen: String,
    pub image: Option<FwFile>,
}

impl From<&catalog::Entry> for CatalogEntry {
    fn from(e: &catalog::Entry) -> Self {
        Self {
            file_name: e.file_name.clone(),
            channel: e.channel,
            size: e.size,
            first_seen: e.date.clone(),
            image: None,
        }
    }
}

/// Read the firmware catalogue.
pub fn load_catalogue(user_dir: &Path) -> Result<catalog::Catalog> {
    catalog::load(user_dir).context("could not read the firmware catalogue")
}

/// When each channel was last polled by whoever generated the index.
pub fn channel_freshness(catalog: &catalog::Catalog) -> Vec<String> {
    catalog
        .channel_checked_at
        .iter()
        .map(|(channel, at)| format!("{channel} last checked {at}"))
        .collect()
}

/// Fetch a chosen build to disk and verify it against the catalogue.
pub fn fetch_and_verify(
    chosen: &catalog::Entry,
    report: &mut dyn FnMut(FlashProgress),
) -> Result<FwFile> {
    let path = materialise(chosen, report)?;
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;

    report(FlashProgress::note(
        FlashStage::Verifying,
        format!("checking {} against the catalogue", chosen.file_name),
    ));
    verify_against_catalogue(chosen, &bytes, &path, report)?;

    report(FlashProgress::note(
        FlashStage::Parsing,
        "parsing the container",
    ));
    fw::FwFile::parse(bytes).map_err(|e| {
        e.context(format!(
            "the selected file ({}) is not a usable firmware image",
            path.display()
        ))
    })
}

/// Provision the puck and flash the image, reporting as it goes.
pub fn flash_firmware(config: FlashConfig) -> Result<FlashResult> {
    let mut nop = NopReporter;
    let reporter = config.progress.unwrap_or(&mut nop);
    let serial = config.serial;
    let entry = config.entry;

    reporter.report(FlashProgress::note(
        FlashStage::Provisioning,
        "Entering bootloader",
    ));

    let mut notes = |m: &str| reporter.report(FlashProgress::note(FlashStage::Provisioning, m));

    let port = match bootloader::find_cdc(bootloader::VID, bootloader::PID_BOOTLOADER) {
        Some(p) => {
            notes(&format!("already in bootloader mode ({p})"));
            p
        }
        None => {
            reboot::enter_bootloader(&mut notes)?;
            bootloader::find_cdc(bootloader::VID, bootloader::PID_BOOTLOADER)
                .context("the bootloader enumerated but its CDC port is gone already")?
        }
    };

    let mut bl = bootloader::Bootloader::open(&port)?;

    let blob = provision::build_provisioning(&serial)?;
    notes(&format!(
        "magic 0x{:08X} (the application-required generic)",
        provision::PROVISIONING_MAGIC
    ));
    notes(&format!("hardware id {}", provision::PUCK_HARDWARE_ID));
    notes(&format!("serial {serial}"));
    bl.write_provisioning(&blob)?;
    notes("reading the provisioning block back...");
    let info = bl.info()?;
    let view = provision::locate_provisioning(&info).with_context(|| {
        format!(
            "the bootloader returned no provisioning block\n--- MESSAGE_INFO reply ---\n{}",
            fw::hex_dump(&info)
        )
    })?;
    if !view.is_identical_to(&blob) {
        bail!(
            "provisioning verification failed: the block read back is not the one written \
             (serial {:?}, hardware id {})",
            view.serial(),
            view.hardware_id()
        );
    }
    notes(&format!(
        "confirmed: serial {} hardware id {} CRC 0x{:08X}",
        view.serial(),
        view.hardware_id(),
        view.crc32()
    ));

    notes(&format!(
        "flashing {} ({}), first seen {}, {}",
        entry.file_name,
        fw::human_size(entry.size),
        if entry.first_seen.is_empty() {
            "unknown"
        } else {
            &entry.first_seen
        },
        entry.channel.tag()
    ));

    let payload = entry
        .image
        .as_ref()
        .expect("image was loaded before flashing")
        .payload();
    let header = fw::build_header(payload);
    let total_chunks = payload.len().div_ceil(bootloader::CHUNK_SIZE) as u32;

    reporter.report(FlashProgress {
        stage: FlashStage::Flashing,
        chunk: Some(0),
        total_chunks: Some(total_chunks),
        bytes_written: Some(0),
        total_bytes: Some(payload.len() as u64),
        message: "Flashing".into(),
    });

    let payload_len = payload.len();
    bl.flash_with(payload, &header, |p| {
        if let Some(note) = p.note {
            reporter.report(FlashProgress::note(FlashStage::Flashing, note));
        } else {
            reporter.report(FlashProgress {
                stage: FlashStage::Flashing,
                chunk: Some(p.chunk as u32),
                total_chunks: Some(p.chunks as u32),
                bytes_written: Some(p.bytes as u64),
                total_bytes: Some(p.total_bytes as u64),
                message: format!("{} of {} chunks", p.chunk, p.chunks),
            });
        }
    })?;

    reporter.report(FlashProgress::note(
        FlashStage::Resetting,
        "Resetting the device",
    ));
    bl.reset()?;

    reporter.report(FlashProgress::note(FlashStage::Done, "done"));

    Ok(FlashResult {
        serial,
        firmware_name: entry.file_name.clone(),
        channel: entry.channel,
        payload_size: payload_len as u64,
    })
}
