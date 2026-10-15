//! Getting from a running application into its bootloader.

use anyhow::{bail, Context, Result};
use std::time::Duration;

use super::bootloader::{find_cdc, trace, PID_APPLICATION, PID_BOOTLOADER, VID};

/// How long to wait for the device to show up in bootloader mode.
const RE_ENUMERATE_TIMEOUT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(400);

/// Ask the running application to jump into its bootloader.
pub fn enter_bootloader(note: &mut dyn FnMut(&str)) -> Result<String> {
    let api = hidapi::HidApi::new().context("enumerating HID")?;
    let mut sent = false;

    for dev in api.device_list() {
        if dev.vendor_id() != VID || dev.product_id() != PID_APPLICATION {
            continue;
        }
        if dev.usage_page() != 0xFF00 {
            continue;
        }
        let handle = dev
            .open_device(&api)
            .with_context(|| format!("opening HID {}", dev.path().to_string_lossy()))?;

        let mut report = [0u8; 64];
        report[0] = 2;
        report[1] = 0x90;
        handle.send_feature_report(&report)?;
        sent = true;
        break;
    }

    if !sent {
        bail!(
            "no running puck found on {VID:04x}:{PID_APPLICATION:04x} with its vendor HID interface.\n\
             Is the controller connected, and is it powered?"
        );
    }

    note("requested REBOOT_TO_ISP over HID");

    let deadline = std::time::Instant::now() + RE_ENUMERATE_TIMEOUT;
    loop {
        if let Some(port) = find_cdc(VID, PID_BOOTLOADER) {
            note(&format!("bootloader enumerated on {port}"));
            trace(&format!("bootloader CDC on {port}"));
            return Ok(port);
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "the puck did not re-enumerate as a bootloader within {}s",
                RE_ENUMERATE_TIMEOUT.as_secs()
            );
        }
        std::thread::sleep(POLL);
    }
}
