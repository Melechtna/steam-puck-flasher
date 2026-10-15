//! Background work. Nothing in here directly touches the gui.

use std::sync::mpsc::Sender;
use std::sync::Arc;

use anyhow::Context;

use crate::flasher::{
    catalog, channel_freshness, fetch_and_verify, flash_firmware, fw, load_catalogue,
    user_firmware_dir, CatalogEntry, FlashConfig, FlashProgress, FlashResult, FlashStage,
    ProgressReporter,
};
use crate::models::Level;

/// Everything a worker can tell the UI.
pub enum Event {
    Catalogue(Arc<catalog::Catalog>),
    CatalogueFailed(String),
    Line(Level, String),
    Progress(FlashProgress),
    Finished(FlashResult),
    Failed(String),
}

/// A progress report with no counters.
fn beat(stage: FlashStage, message: impl Into<String>) -> Event {
    Event::Progress(FlashProgress::note(stage, message))
}

/// Read the upstream index plus any local builds. Needs the network.
pub fn spawn_catalogue(tx: Sender<Event>) {
    std::thread::spawn(move || {
        let _ = tx.send(beat(
            FlashStage::ReadingCatalogue,
            "Fetching the firmware catalogue",
        ));
        match load_catalogue(&user_firmware_dir()) {
            Ok(c) => {
                let n = c.entries.len();
                let at = c.index_generated_at.clone();
                let fresh = channel_freshness(&c);
                let _ = tx.send(Event::Catalogue(Arc::new(c)));
                let _ = tx.send(Event::Line(Level::Ok, format!("{n} builds available")));
                if !at.is_empty() {
                    let _ = tx.send(Event::Line(
                        Level::Info,
                        format!("upstream index generated {at}"),
                    ));
                }
                for line in fresh {
                    let _ = tx.send(Event::Line(Level::Info, line));
                }
            }
            Err(e) => {
                let msg = format!("{e:#}");
                let _ = tx.send(Event::CatalogueFailed(msg));
            }
        }
    });
}

/// Verify the chosen build, then provision and flash it.
pub fn spawn_flash(tx: Sender<Event>, cat: Arc<catalog::Catalog>, index: usize, serial: String) {
    std::thread::spawn(move || match flash(tx.clone(), &cat, index, &serial) {
        Ok(result) => {
            let _ = tx.send(Event::Finished(result));
        }
        Err(e) => {
            let _ = tx.send(Event::Failed(format!("{e:#}")));
        }
    });
}

fn flash(
    tx: Sender<Event>,
    cat: &catalog::Catalog,
    index: usize,
    serial: &str,
) -> anyhow::Result<FlashResult> {
    let chosen = cat
        .entries
        .get(index)
        .context("that build is no longer in the catalogue")?;

    let mut report = |p: FlashProgress| {
        let _ = tx.send(Event::Progress(p));
    };
    let image = fetch_and_verify(chosen, &mut report)?;
    for line in describe(&image) {
        let _ = tx.send(Event::Line(Level::Info, line));
    }

    let mut entry = CatalogEntry::from(chosen);
    entry.image = Some(image);

    let mut reporter = TxReporter(tx);
    flash_firmware(FlashConfig {
        serial: serial.to_string(),
        entry,
        progress: Some(&mut reporter),
    })
}

/// Adapts the core's progress callback onto the channel.
struct TxReporter(Sender<Event>);

impl ProgressReporter for TxReporter {
    fn report(&mut self, p: FlashProgress) {
        let _ = self.0.send(Event::Progress(p));
    }
}

impl From<FlashProgress> for Event {
    fn from(p: FlashProgress) -> Self {
        Event::Progress(p)
    }
}

/// Serial numbers of any pucks currently plugged in.
pub fn detect_serials() -> Vec<String> {
    use serialport::SerialPortType::UsbPort;

    let Ok(ports) = serialport::available_ports() else {
        return Vec::new();
    };
    ports
        .into_iter()
        .filter_map(|p| match p.port_type {
            UsbPort(info) if info.vid == 0x28de => Some(info),
            _ => None,
        })
        .filter_map(|info| info.serial_number)
        .collect()
}

/// Show the sizes the user cares about.
pub fn describe(image: &fw::FwFile) -> [String; 3] {
    [
        format!("magic        0x{:08X}", image.magic),
        format!(
            "image        {} KiB total, {} KiB payload",
            (image.payload_size as usize + fw::HEADER_LEN).div_ceil(1024),
            image.payload_size.div_ceil(1024)
        ),
        format!("payload CRC  0x{:08X}", image.payload_crc32),
    ]
}
