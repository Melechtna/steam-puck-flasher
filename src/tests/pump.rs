//! The worker event pump.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::app::App;
use crate::flasher::catalog::{Catalog, Channel, Entry, Source};
use crate::flasher::{FlashProgress, FlashResult, FlashStage};
use crate::models::{Level, Step};
use crate::worker::Event;

fn entry(name: &str, channel: Channel) -> Entry {
    Entry {
        file_name: name.into(),
        channel,
        date: "2024-03-01T00:00:00Z".into(),
        size: 2048,
        crc32: 0x1234_5678,
        sha256: "b".repeat(64),
        source: Source::Remote {
            url: format!("https://example.invalid/{name}"),
        },
    }
}

fn catalogue(index_generated_at: &str) -> Arc<Catalog> {
    Arc::new(Catalog {
        entries: vec![
            entry("BETA_0001.fw", Channel::Beta),
            entry("STABLE_0001.fw", Channel::Stable),
        ],
        index_generated_at: index_generated_at.into(),
        channel_checked_at: BTreeMap::new(),
    })
}

fn chunk(chunk: u32, chunks: u32) -> Event {
    Event::Progress(FlashProgress {
        stage: FlashStage::Flashing,
        chunk: Some(chunk),
        total_chunks: Some(chunks),
        bytes_written: Some(chunk as u64 * 1000),
        total_bytes: Some(chunks as u64 * 1000),
        message: format!("{chunk} of {chunks} chunks"),
    })
}

#[test]
fn events_arrive_over_the_channel() {
    let mut app = App::bare();
    app.outbox
        .send(Event::Line(Level::Info, "hello".into()))
        .unwrap();
    app.pump();

    assert_eq!(app.log.len(), 1);
    assert_eq!(app.log[0].text, "hello");
}

#[test]
fn a_catalogue_populates_the_rows_in_index_order() {
    let mut app = App::bare();
    app.outbox
        .send(Event::Catalogue(catalogue("2024-03-02T00:00:00Z")))
        .unwrap();
    app.pump();

    assert_eq!(app.rows.len(), 2);
    assert_eq!(
        app.rows[0].file_name, "BETA_0001.fw",
        "the core sorts newest first"
    );
    assert!(app.catalogue.is_some());
}

#[test]
fn a_stale_index_is_flagged_as_a_warning() {
    let mut app = App::bare();
    app.outbox.send(Event::Catalogue(catalogue(""))).unwrap();
    app.pump();

    assert!(
        app.log.iter().any(|l| l.level == Level::Warn),
        "an index with no timestamp must say channel membership may be stale"
    );
}

#[test]
fn a_catalogue_failure_is_shown_not_swallowed() {
    let mut app = App::bare();
    app.outbox
        .send(Event::CatalogueFailed("no network".into()))
        .unwrap();
    app.pump();

    assert_eq!(app.catalogue_error, "no network");
    assert!(app.log.iter().any(|l| l.level == Level::Error));
}

#[test]
fn finishing_moves_to_the_result_step_with_a_full_meter() {
    let mut app = App::bare();
    app.outbox
        .send(Event::Finished(FlashResult {
            serial: "FXB996010210B".into(),
            firmware_name: "STABLE_0001.fw".into(),
            channel: Channel::Stable,
            payload_size: 100 * 1024,
        }))
        .unwrap();
    app.pump();

    assert_eq!(app.step, Step::Done);
    assert!(app.failure.is_none());
    assert_eq!(app.meter.fraction, 1.0);
    let said = app.result.as_ref().unwrap();
    assert!(said.contains("STABLE_0001.fw"), "got {said}");
    assert!(said.contains("FXB996010210B"), "got {said}");
}

#[test]
fn failing_keeps_the_reason_and_claims_no_result() {
    let mut app = App::bare();
    app.outbox
        .send(Event::Failed(
            "CRC32 mismatch: expected 0x1, got 0x2".into(),
        ))
        .unwrap();
    app.pump();

    assert_eq!(app.step, Step::Done);
    assert_eq!(
        app.failure.as_deref(),
        Some("CRC32 mismatch: expected 0x1, got 0x2")
    );
    assert!(app.result.is_none());
}

#[test]
fn chunk_reports_move_the_meter_without_flooding_the_log() {
    let mut app = App::bare();
    for c in 0..50 {
        app.outbox.send(chunk(c, 50)).unwrap();
    }
    app.pump();

    assert_eq!(
        app.log.len(),
        0,
        "50 chunk reports must not become 50 log lines"
    );
    assert_eq!(app.meter.total_chunks, 50);
    assert_eq!(app.meter.chunk, 49);
    assert!((app.meter.fraction - 0.98).abs() < 1e-6);
}

#[test]
fn repeated_stage_messages_are_logged_once() {
    let mut app = App::bare();
    let stage = |msg: &str| {
        Event::Progress(FlashProgress {
            stage: FlashStage::Provisioning,
            chunk: None,
            total_chunks: None,
            bytes_written: None,
            total_bytes: None,
            message: msg.into(),
        })
    };
    app.outbox.send(stage("reading back...")).unwrap();
    app.outbox.send(stage("reading back...")).unwrap();
    app.outbox.send(stage("confirmed: serial FXB1")).unwrap();
    app.pump();

    let texts: Vec<_> = app.log.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(texts, ["reading back...", "confirmed: serial FXB1"]);
}

#[test]
fn a_failure_after_progress_is_not_overwritten_by_a_later_success() {
    let mut app = App::bare();
    app.outbox.send(chunk(10, 50)).unwrap();
    app.outbox
        .send(Event::Failed("verification failed".into()))
        .unwrap();
    app.pump();

    assert!(app.result.is_none());
    assert_eq!(app.failure.as_deref(), Some("verification failed"));
}

#[test]
fn draining_an_empty_channel_keeps_the_receiver() {
    let mut app = App::bare();
    app.pump();

    assert!(
        app.inbox.is_some(),
        "the receiver must be put back every frame"
    );
    assert!(app.log.is_empty());
}
