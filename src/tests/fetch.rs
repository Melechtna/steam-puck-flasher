//! Where downloaded builds are allowed to land.

use crate::flasher::catalog::{Channel, Entry, Source};
use crate::flasher::fetch::{scratch_dir, user_firmware_dir, verify_against_catalogue};
use crate::flasher::fw::{build_header, crc32, sha256};
use crate::flasher::{FlashProgress, FlashStage};

#[test]
fn downloads_go_to_the_temp_directory() {
    let scratch = scratch_dir();
    let temp = std::env::temp_dir();
    assert!(
        scratch.starts_with(&temp),
        "scratch dir {scratch:?} is not under the temp dir {temp:?}"
    );
}

#[test]
fn downloads_never_land_where_user_builds_are_scanned() {
    let scratch = scratch_dir();
    let user_dir = user_firmware_dir();

    assert!(
        !scratch.starts_with(&user_dir),
        "a download into {scratch:?} would be re-reported as a user build from {user_dir:?}"
    );
    assert_ne!(scratch, user_dir);
}

#[test]
fn the_two_folders_have_distinct_names() {
    assert!(user_firmware_dir().ends_with("firmware"));
    assert!(!scratch_dir().ends_with("firmware"));
}

/// A container wrapping a payload.
fn wrapped(payload: &[u8]) -> Vec<u8> {
    let mut bytes = build_header(payload).to_vec();
    bytes.extend_from_slice(payload);
    bytes
}

fn entry_naming(crc: u32, sha: String) -> Entry {
    Entry {
        file_name: "TEST.fw".into(),
        channel: Channel::Stable,
        date: "2026-10-04".into(),
        size: 0,
        crc32: crc,
        sha256: sha,
        source: Source::Remote {
            url: "https://example.invalid/TEST.fw".into(),
        },
    }
}

fn verify(entry: &Entry, bytes: &[u8]) -> anyhow::Result<()> {
    let mut sink = |_: FlashProgress| {};
    verify_against_catalogue(entry, bytes, std::path::Path::new("TEST.fw"), &mut sink)
}

#[test]
fn crc32_covers_the_payload_but_sha256_covers_the_whole_file() {
    let payload = b"an arm image would go here".repeat(100);
    let bytes = wrapped(&payload);

    assert_ne!(crc32(&payload), crc32(&bytes));
    assert_ne!(sha256(&payload), sha256(&bytes));

    let entry = entry_naming(crc32(&payload), sha256(&bytes));
    verify(&entry, &bytes).expect("a file matching the index's own definition must verify");
}

#[test]
fn hashing_the_payload_for_sha256_is_rejected() {
    let payload = b"an arm image would go here".repeat(100);
    let bytes = wrapped(&payload);

    let wrong = entry_naming(crc32(&payload), sha256(&payload));
    let err = verify(&wrong, &bytes).unwrap_err().to_string();
    assert!(err.contains("SHA256 mismatch"), "got: {err}");
}

#[test]
fn a_flipped_byte_anywhere_is_caught() {
    let payload = b"an arm image would go here".repeat(100);
    let good = wrapped(&payload);
    let entry = entry_naming(crc32(&payload), sha256(&good));

    let mut tampered = good.clone();
    tampered[20] ^= 0x01;
    let err = verify(&entry, &tampered).unwrap_err().to_string();
    assert!(err.contains("SHA256 mismatch"), "got: {err}");
}

#[test]
fn a_local_build_is_judged_only_on_its_own_container_crc() {
    let payload = b"an arm image would go here".repeat(100);
    let bytes = wrapped(&payload);

    let entry = entry_naming(0, String::new());
    verify(&entry, &bytes).expect("a self-consistent local build must verify");
}

#[test]
fn a_local_build_with_a_lie_in_its_header_is_rejected() {
    let payload = b"an arm image would go here".repeat(100);
    let mut bytes = wrapped(&payload);
    bytes[8..12].copy_from_slice(&crc32(b"something else").to_le_bytes());

    let entry = entry_naming(0, String::new());
    let err = verify(&entry, &bytes).unwrap_err().to_string();
    assert!(err.contains("corrupt download"), "got: {err}");
}

#[test]
fn verification_reports_the_stage_it_reached() {
    let payload = b"an arm image would go here".repeat(100);
    let bytes = wrapped(&payload);
    let entry = entry_naming(crc32(&payload), sha256(&bytes));

    let mut seen = Vec::new();
    let mut sink = |p: FlashProgress| seen.push((p.stage, p.message));
    verify_against_catalogue(&entry, &bytes, std::path::Path::new("TEST.fw"), &mut sink).unwrap();

    assert!(
        seen.iter()
            .any(|(s, m)| *s == FlashStage::Verifying && m.contains("verified")),
        "nothing reported success: {seen:?}"
    );
}
