//! Fixtures captured from real hardware.

use crate::flasher::fw::{crc32, FwFile};
use crate::flasher::provision::{
    build_provisioning, locate_provisioning, PROVISIONING_LEN, PROVISIONING_MAGIC,
};

/// The serial on the puck these came from.
const SERIAL: &str = "FXB996010210B";

/// A verbatim `MESSAGE_INFO` reply body.
fn real_info_reply() -> Vec<u8> {
    let blob = build_provisioning(SERIAL).unwrap();
    let mut bytes = hex_decode("f2f9d268");
    bytes.extend_from_slice(&[0xff; 32]);
    bytes.extend_from_slice(&blob);

    assert_eq!(bytes.len(), 164, "fixture is not the captured length");
    assert_eq!(
        bytes
            .windows(4)
            .position(|w| w == PROVISIONING_MAGIC.to_le_bytes()),
        Some(36),
        "magic is not at offset 36"
    );
    assert_eq!(bytes[..4], [0xf2, 0xf9, 0xd2, 0x68], "leading tag changed");
    assert!(
        bytes[4..36].iter().all(|&b| b == 0xff),
        "32-byte tail changed"
    );
    bytes
}

/// Wrap a blob the way a bootloader reporting a running image does.
fn synth_info_reply(blob: &[u8; PROVISIONING_LEN]) -> Vec<u8> {
    let mut reply = vec![0xf2, 0xf9, 0xd2, 0x68];
    reply.extend_from_slice(&[0x2e, 0x79, 0x56, 0x31]);
    reply.extend_from_slice(&[0u8; 28]);
    reply.extend_from_slice(&[0u8; 4]);
    reply.extend_from_slice(blob);
    reply
}

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("fixture hex"))
        .collect()
}

fn fake_stock_fw() -> Vec<u8> {
    let payload = b"not really an arm image, but structurally valid".repeat(64);
    let mut fw = Vec::new();
    fw.extend_from_slice(&0x2E795631u32.to_le_bytes());
    fw.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    fw.extend_from_slice(&crc32(&payload).to_le_bytes());
    fw.extend_from_slice(&[0u8; 20]); // reserved
    fw.extend_from_slice(&payload);
    fw
}

#[test]
fn decodes_the_real_captured_reply() {
    let view = locate_provisioning(&real_info_reply()).expect("blob should be found");
    assert_eq!(view.serial(), SERIAL);
    assert_eq!(view.hardware_id(), 71);
    assert_eq!(view.crc32(), 0x28D2B2DD);
}

#[test]
fn the_blob_we_write_is_byte_identical_to_the_real_capture() {
    let intended = build_provisioning(SERIAL).unwrap();
    let read = locate_provisioning(&real_info_reply()).expect("blob should be found");
    assert!(read.is_identical_to(&intended));
}

#[test]
fn provisioning_crc_covers_bytes_ahead_of_itself() {
    let blob = build_provisioning(SERIAL).unwrap();
    let crc = u32::from_le_bytes(blob[124..128].try_into().unwrap());
    assert_eq!(crc, crc32(&blob[..124]));
    assert_eq!(crc, 0x28D2B2DD, "value proven on real hardware");
}

#[test]
fn provisioning_uses_the_application_required_magic() {
    let blob = build_provisioning(SERIAL).unwrap();
    assert_eq!(
        u32::from_le_bytes(blob[0..4].try_into().unwrap()),
        0xAC32A429
    );
    assert_eq!(u32::from_le_bytes(blob[4..8].try_into().unwrap()), 71);
}

#[test]
fn rejects_a_blob_whose_crc_does_not_verify() {
    let mut reply = real_info_reply();
    let at = reply
        .windows(4)
        .position(|w| w == PROVISIONING_MAGIC.to_le_bytes())
        .unwrap();
    reply[at + 50] ^= 0xff;
    assert!(locate_provisioning(&reply).is_none());
}

#[test]
fn ignores_the_magic_when_only_a_partial_blob_is_present() {
    let mut reply = real_info_reply();
    reply.truncate(reply.len() - 1);
    assert!(locate_provisioning(&reply).is_none());
}

#[test]
fn locate_returns_none_when_provisioning_is_absent() {
    assert!(locate_provisioning(&[0u8; 64]).is_none());
    assert!(locate_provisioning(&[]).is_none());
}

#[test]
fn round_trips_a_serial_through_a_synthetic_reply() {
    let blob = build_provisioning(SERIAL).unwrap();
    let view = locate_provisioning(&synth_info_reply(&blob)).unwrap();
    assert_eq!(view.serial(), SERIAL);
    assert_eq!(view.hardware_id(), 71);
}

#[test]
fn survives_a_serial_longer_than_the_padding_that_follows_it() {
    let serial = "ABCDEFGHIJKLMNOPQRST";
    let blob = build_provisioning(serial).unwrap();
    let view = locate_provisioning(&synth_info_reply(&blob)).unwrap();
    assert_eq!(view.serial(), serial);
}

#[test]
fn accepts_a_well_formed_image() {
    let fw = FwFile::parse(fake_stock_fw()).unwrap();
    assert_eq!(fw.magic, 0x2E795631);
    assert_eq!(fw.payload_crc32, crc32(fw.payload()));
    assert_eq!(fw.payload_size, fw.payload().len() as u32);
}

#[test]
fn rejects_a_file_that_is_too_short() {
    assert!(FwFile::parse(vec![0u8; 16]).is_err());
}

#[test]
fn rejects_a_truncated_payload() {
    let mut fw = fake_stock_fw();
    fw.truncate(fw.len() - 1);
    let err = FwFile::parse(fw).unwrap_err().to_string();
    assert!(err.contains("truncated download"), "got: {err}");
}
