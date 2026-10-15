//! The CLI

use std::io::{self, BufRead, Write};

use anyhow::{bail, Context, Result};

use crate::flasher::{self, catalog, FlashConfig, FlashProgress, FlashStage, ProgressReporter};

/// Runs the whole flow: serial, pick a build, flash.
pub fn run() -> Result<()> {
    println!("steam-puck-flasher -- Valve Steam Controller 2 puck (Proteus)");
    println!();

    let catalog = load()?;
    let serial = ask_serial()?;
    let index = pick(&catalog)?;
    let chosen = &catalog.entries[index];

    println!();
    let mut reporter = StdoutReporter;
    let image = flasher::fetch_and_verify(chosen, &mut |p| {
        println!("  {}", p.message);
    })?;

    println!();
    println!("  serial    {serial}");
    println!(
        "  build     {} ({})",
        chosen.file_name,
        chosen.channel.tag()
    );
    println!(
        "  payload   {} ({} bytes)",
        flasher::fw::human_size(image.payload().len() as u64),
        image.payload().len()
    );
    println!("  crc32     0x{:08X}", image.payload_crc32);

    println!();
    println!("About to flash {} to {serial}.", chosen.file_name);
    println!("This erases the puck's application flash. There is no undo.");
    if !confirm("Continue? [y/N] ") {
        println!("cancelled.");
        return Ok(());
    }

    let mut entry = flasher::CatalogEntry::from(chosen);
    entry.image = Some(image);

    let config = FlashConfig {
        serial,
        entry,
        progress: Some(&mut reporter),
    };

    match flasher::flash_firmware(config) {
        Ok(result) => {
            println!();
            println!(
                "Done. {} ({}) is now running on {}.",
                result.firmware_name,
                result.channel.tag(),
                result.serial
            );
            Ok(())
        }
        Err(e) => {
            println!();
            eprintln!("Flash failed: {e:#}");
            bail!("nothing was left half-written")
        }
    }
}

fn load() -> Result<catalog::Catalog> {
    let dir = flasher::user_firmware_dir();
    println!(
        "Reading the firmware catalogue (user builds from {})...",
        dir.display()
    );

    let catalog = flasher::load_catalogue(&dir)?;
    if catalog.entries.is_empty() {
        bail!(
            "no firmware found upstream, and {} holds no .fw files",
            dir.display()
        );
    }

    println!("{} builds available.", catalog.entries.len());
    if !catalog.index_generated_at.is_empty() {
        println!("upstream index generated {}", catalog.index_generated_at);
    }
    Ok(catalog)
}

fn ask_serial() -> Result<String> {
    println!();
    if let Some(found) = crate::worker::detect_serials().first() {
        println!("A puck on the bus reports serial {found}.");
    }
    print!("Serial number: ");
    io::stdout().flush().ok();

    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .context("reading stdin")?;
    let serial = line.trim().to_uppercase();

    if serial.is_empty() {
        bail!("a serial number is required");
    }
    if let Some(bad) = serial.chars().find(|c| !c.is_ascii_alphanumeric()) {
        bail!("'{bad}' is not allowed in a serial -- letters and digits only");
    }
    if serial.len() > 20 {
        bail!(
            "that is {} characters; a serial is at most 20",
            serial.len()
        );
    }

    println!("Serial: {serial}");
    Ok(serial)
}

fn pick(catalog: &catalog::Catalog) -> Result<usize> {
    println!();
    for (i, e) in catalog.entries.iter().enumerate() {
        let date = if e.date.is_empty() {
            "unknown     "
        } else {
            &e.date[..e.date.len().min(10)]
        };
        println!(
            "  {:>3}  {:<10}  ({})  {:>8}  {}",
            i + 1,
            date,
            e.channel.tag(),
            crate::flasher::fw::human_size(e.size),
            e.file_name
        );
    }

    loop {
        print!("\nWhich build? [1-{}]: ", catalog.entries.len());
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().lock().read_line(&mut line)? == 0 {
            bail!("no input. aborting.");
        }
        match line.trim().parse::<usize>() {
            Ok(n) if n >= 1 && n <= catalog.entries.len() => return Ok(n - 1),
            Ok(n) => println!("there is no build {n}."),
            Err(_) => println!("that is not a number."),
        }
    }
}

fn confirm(question: &str) -> bool {
    print!("{question}");
    io::stdout().flush().ok();

    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Prints progress as it arrives.
struct StdoutReporter;

impl ProgressReporter for StdoutReporter {
    fn report(&mut self, p: FlashProgress) {
        match p.stage {
            FlashStage::Flashing => {
                if let Some(f) = p.fraction() {
                    let done = (f * 24.0).round() as usize;
                    let bar = format!("[{}{}]", "#".repeat(done), ".".repeat(24 - done));
                    match (p.chunk, p.total_chunks) {
                        (Some(c), Some(t)) => println!("  {bar} {f:.0}%  chunk {c}/{t}"),
                        _ => println!("  {bar} {f:.0}%"),
                    }
                } else {
                    println!("  {}", p.message);
                }
            }
            _ => println!("  {}", p.message),
        }
    }
}
