//! Valve bootloader transport.

use anyhow::{bail, Context, Result};
use serialport::{SerialPort, SerialPortType};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use super::hdlc::{self, Decoder};

/// USB identities.
pub const VID: u16 = 0x28DE;
/// The running application enumerates as this.
pub const PID_APPLICATION: u16 = 0x1304;
/// The bootloader enumerates as this.
pub const PID_BOOTLOADER: u16 = 0x1007;

/// Bootloader message opcodes.
const MSG_INFO: u16 = 0x1233;
const MSG_UICR_PROVISION: u16 = 0x1238;
const MSG_RESET: u16 = 0x1237;

/// Key that unlocks the provisioning slot in UICR.
const UICR_PROVISION_KEY: u32 = 0xE86DA4C7;

/// Payload bytes per FW_DATA frame.
pub const CHUNK_SIZE: usize = 32 * 1024;

/// The bootloader is not high bandwidth, 10s just to be safe.
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) fn is_silence(e: &anyhow::Error) -> bool {
    e.chain()
        .any(|c| c.to_string().starts_with("timed out after "))
}

pub fn trace(line: &str) {
    if std::env::var_os("STEAM_PUCK_FLASHER_TRACE").is_some() {
        eprintln!("[trace] {line}");
    }
}

/// How far a flash has gone between chunks.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub chunk: usize,
    pub chunks: usize,
    pub bytes: usize,
    pub total_bytes: usize,
    pub note: Option<String>,
}

impl Progress {
    pub fn saying(note: impl Into<String>) -> Self {
        Self {
            note: Some(note.into()),
            ..Default::default()
        }
    }
}

/// An open handle to the bootloader CDC interface.
pub struct Bootloader {
    port: Box<dyn SerialPort>,
    decoder: Decoder,
}

/// Locate the CDC ACM port belonging to a given USB vid:pid.
pub fn find_cdc(vid: u16, pid: u16) -> Option<String> {
    for port in serialport::available_ports().ok()? {
        if let SerialPortType::UsbPort(info) = port.port_type {
            if info.vid == vid && info.pid == pid {
                return Some(port.port_name);
            }
        }
    }
    None
}

impl Bootloader {
    pub fn open(port_name: &str) -> Result<Self> {
        let port = serialport::new(port_name, 115200)
            .timeout(Duration::from_millis(500))
            .open()
            .with_context(|| format!("opening {port_name}"))?;
        Ok(Self {
            port,
            decoder: Decoder::new(),
        })
    }

    pub(super) fn request(&mut self, msg: &[u8], timeout: Duration) -> Result<Vec<u8>> {
        let framed = hdlc::encode(msg);
        trace(&format!(
            ">> {} bytes framed -> {} on the wire",
            msg.len(),
            framed.len()
        ));
        self.port
            .write_all(&framed)
            .context("writing to bootloader")?;
        self.port.flush().ok();

        let deadline = Instant::now() + timeout;
        let mut buf = [0u8; 1024];
        loop {
            if Instant::now() >= deadline {
                bail!(
                    "timed out after {timeout:?} waiting for bootloader reply to opcode 0x{}{:02x}",
                    msg.first().copied().unwrap_or(0),
                    msg.get(1).copied().unwrap_or(0),
                );
            }
            match self.port.read(&mut buf) {
                Ok(0) => std::thread::sleep(Duration::from_millis(5)),
                Ok(n) => match self.take_reply(&buf[..n])? {
                    Some(body) => return Ok(body),
                    None => continue,
                },
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e).context("reading from bootloader"),
            }
        }
    }

    /// Interpret a read.
    fn take_reply(&mut self, bytes: &[u8]) -> Result<Option<Vec<u8>>> {
        let frames = self.decoder.push(bytes);

        for frame in &frames {
            match frame.first() {
                Some(0x00) => trace(&format!("<< ACK, {} byte body", frame.len() - 1)),
                Some(st) => trace(&format!("<< REFUSED, status 0x{st:02X}")),
                None => trace("<< empty frame"),
            }
        }

        if let Some(&st) = frames.iter().find_map(|f| f.first()).filter(|&&s| s != 0) {
            bail!("bootloader refused the request (status 0x{st:02X})");
        }

        Ok(frames
            .into_iter()
            .find(|f| f.first() == Some(&0x00))
            .map(|f| f[1..].to_vec()))
    }

    /// Ask the bootloader to describe itself.
    pub fn info(&mut self) -> Result<Vec<u8>> {
        self.request(&MSG_INFO.to_le_bytes(), REPLY_TIMEOUT)
    }

    /// Write the 128-byte provisioning blob into UICR.
    pub fn write_provisioning(&mut self, blob: &[u8; 128]) -> Result<()> {
        let mut msg = Vec::with_capacity(4 + 4 + 128);
        msg.extend_from_slice(&MSG_UICR_PROVISION.to_le_bytes());
        msg.extend_from_slice(&UICR_PROVISION_KEY.to_le_bytes());
        msg.extend_from_slice(&blob[..]);
        self.request(&msg, REPLY_TIMEOUT)?;
        Ok(())
    }

    /// Leave the bootloader and start the application.
    pub fn reset(&mut self) -> Result<()> {
        self.request(&MSG_RESET.to_le_bytes(), REPLY_TIMEOUT)?;
        Ok(())
    }
}
