//! The firmware catalogue.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const INDEX_URL: &str =
    "https://raw.githubusercontent.com/OpenSteamController/Ibex-Firmware/main/index.json";
const RAW_BASE: &str =
    "https://raw.githubusercontent.com/OpenSteamController/Ibex-Firmware/main/Puck";

/// Channel name in the index for Valve's shipping builds.
pub const CHANNEL_STABLE_NAME: &str = "stable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
    User,
}

impl Channel {
    pub fn tag(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::User => "user",
        }
    }
}

/// Where the bytes will come from once chosen.
#[derive(Clone, Debug)]
pub enum Source {
    Remote { url: String },
    Local { path: PathBuf },
}

pub struct Entry {
    pub file_name: String,
    pub channel: Channel,
    pub date: String,
    pub size: u64,
    pub crc32: u32,
    pub sha256: String,
    pub source: Source,
}

pub struct Catalog {
    pub entries: Vec<Entry>,
    pub index_generated_at: String,
    pub channel_checked_at: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Index {
    #[serde(default)]
    puck: BTreeMap<String, Build>,
    #[serde(rename = "generated_at", default)]
    generated_at: String,
    #[serde(rename = "_channels", default)]
    channels: BTreeMap<String, ChannelState>,
}

#[derive(Deserialize, Default)]
struct ChannelState {
    #[serde(default)]
    last_check_at: String,
}

#[derive(Deserialize)]
struct Build {
    #[serde(default)]
    crc32: String,
    #[serde(default)]
    file_size: u64,
    #[serde(default)]
    first_seen: BTreeMap<String, Seen>,
    #[serde(default)]
    sha256: String,
}

#[derive(Deserialize)]
struct Seen {
    #[serde(default)]
    date: String,
}

/// Pull the index and merge it with any local builds.
pub fn load(user_firmware_dir: &Path) -> Result<Catalog> {
    let mut channel_checked_at = BTreeMap::new();

    let mut response = ureq::get(INDEX_URL)
        .header("User-Agent", "steam-puck-flasher")
        .call()
        .context("fetching the Ibex-Firmware index")?;

    let raw = response
        .body_mut()
        .read_to_vec()
        .context("reading the index response")?;

    let index: Index = serde_json::from_slice(&raw).context("parsing the index JSON")?;

    for (name, state) in &index.channels {
        if !state.last_check_at.is_empty() {
            channel_checked_at.insert(name.clone(), state.last_check_at.clone());
        }
    }

    let mut entries = Vec::with_capacity(index.puck.len());

    for (file_name, build) in &index.puck {
        let channel = if build.first_seen.contains_key(CHANNEL_STABLE_NAME) {
            Channel::Stable
        } else {
            Channel::Beta
        };

        let date = build
            .first_seen
            .values()
            .map(|s| s.date.as_str())
            .filter(|d| !d.is_empty())
            .min()
            .unwrap_or("")
            .to_string();

        let crc32 = u32::from_str_radix(&build.crc32, 16).unwrap_or(0);

        entries.push(Entry {
            source: Source::Remote {
                url: format!("{RAW_BASE}/{file_name}"),
            },
            file_name: file_name.clone(),
            channel,
            date,
            size: build.file_size,
            crc32,
            sha256: build.sha256.clone(),
        });
    }

    entries.extend(scan_user_firmware(user_firmware_dir)?);

    entries.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(Catalog {
        entries,
        index_generated_at: index.generated_at,
        channel_checked_at,
    })
}

/// Our own builds, dropped into ./firmware/ next to this executable.
fn scan_user_firmware(dir: &Path) -> Result<Vec<Entry>> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", dir.display())),
    };

    let mut entries = Vec::new();
    for path in read_dir.flatten().map(|e| e.path()) {
        if path.extension().and_then(|e| e.to_str()) != Some("fw") {
            continue;
        }

        let meta = match path.metadata() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("  skipping {}: {e}", path.display());
                continue;
            }
        };

        let stamp: SystemTime = meta
            .created()
            .or_else(|_| meta.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  skipping {}: {e}", path.display());
                continue;
            }
        };

        let crc32 = if bytes.len() >= 12 {
            u32::from_le_bytes(bytes[8..12].try_into().unwrap())
        } else {
            0
        };

        entries.push(Entry {
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string()),
            channel: Channel::User,
            date: super::fw::format_time(stamp),
            size: meta.len(),
            crc32,
            sha256: super::fw::sha256(&bytes),
            source: Source::Local { path },
        });
    }

    Ok(entries)
}
