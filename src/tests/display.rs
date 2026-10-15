//! Display formatting. Dates, sizes, checksums, and the progress meter.

use crate::flasher::catalog::Channel;
use crate::flasher::{FlashProgress, FlashStage};
use crate::models::{Meter, Row};

fn row(channel: Channel, date: &str, size: u64) -> Row {
    Row {
        file_name: "PROTEUS_FW_6A628359.fw".into(),
        channel,
        date: date.into(),
        size,
        crc32: 0xDEAD_BEEF,
        sha256: "a".repeat(64),
    }
}

fn report(stage: FlashStage, bytes: u64, total: u64, chunk: u32, chunks: u32) -> FlashProgress {
    FlashProgress {
        stage,
        chunk: (chunks > 0).then_some(chunk),
        total_chunks: (chunks > 0).then_some(chunks),
        bytes_written: (total > 0).then_some(bytes),
        total_bytes: (total > 0).then_some(total),
        message: format!("{chunk} of {chunks} chunks"),
    }
}

#[test]
fn day_truncates_to_the_date() {
    assert_eq!(
        row(Channel::Stable, "2024-03-01T12:00:00Z", 0).day(),
        "2024-03-01"
    );
}

#[test]
fn day_falls_back_when_the_date_is_missing() {
    assert_eq!(row(Channel::Beta, "", 0).day(), "unknown");
    assert_eq!(row(Channel::Beta, "2024", 0).day(), "unknown");
    assert_eq!(row(Channel::User, "not-a-date-at-all", 0).day(), "unknown");
}

#[test]
fn sizes_and_crcs_are_formatted_for_humans() {
    let r = row(Channel::Stable, "2024-03-01T00:00:00Z", 300 * 1024);
    assert_eq!(r.size_h(), "300 KiB");
    assert_eq!(r.crc_h(), "0xDEADBEEF");
    assert_eq!(row(Channel::User, "", 512).size_h(), "512 B");
}

#[test]
fn a_meter_with_no_chunks_shows_no_chunk_line() {
    assert_eq!(Meter::default().chunk_line(), None);
}

#[test]
fn meter_advances_from_byte_counters() {
    let mut m = Meter::default();
    m.apply(&report(FlashStage::Flashing, 300, 1000, 3, 10));

    assert!((m.fraction - 0.3).abs() < 1e-6, "got {}", m.fraction);
    assert_eq!(m.chunk_line().as_deref(), Some("chunk 3 of 10"));
    assert_eq!(m.headline, "3 of 10 chunks");
}

#[test]
fn meter_clamps_a_silly_ratio() {
    let mut m = Meter::default();
    m.apply(&report(FlashStage::Flashing, 5000, 1000, 0, 0));
    assert_eq!(m.fraction, 1.0);
}

#[test]
fn meter_survives_a_zero_byte_total() {
    let mut m = Meter::default();
    m.apply(&report(FlashStage::Flashing, 0, 0, 0, 0));
    assert_eq!(m.fraction, 0.0, "must not divide by zero");
}

#[test]
fn a_stage_report_without_counters_leaves_the_fraction_alone() {
    let mut m = Meter::default();
    m.apply(&report(FlashStage::Flashing, 250, 1000, 0, 0));
    m.apply(&report(FlashStage::Resetting, 0, 0, 0, 0));
    assert!((m.fraction - 0.25).abs() < 1e-6);
    assert_eq!(m.headline, "0 of 0 chunks");
}
