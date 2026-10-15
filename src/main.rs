//! Flasher for the Valve Steam Controller 2 puck (nRF52833 "Proteus").

mod app;
mod cli;
mod flasher;
mod frame;
mod models;
mod screens;
#[cfg(test)]
mod tests;
mod theme;
mod worker;

use eframe::egui;

/// How the tool was asked to run.
enum Mode {
    Gui,
    Cli,
    Help,
}

fn main() -> anyhow::Result<()> {
    match mode() {
        Mode::Gui => gui(),
        Mode::Cli => cli::run(),
        Mode::Help => {
            println!("{USAGE}");
            Ok(())
        }
    }
}

fn mode() -> Mode {
    let mut args = std::env::args().skip(1);
    let Some(flag) = args.next() else {
        return Mode::Gui;
    };

    match flag.as_str() {
        "-c" | "--cli" => Mode::Cli,
        "-h" | "--help" => Mode::Help,
        "-V" | "--version" => {
            println!("steam-puck-flasher {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        other => {
            eprintln!("unrecognised argument: {other}");
            bail_usage();
        }
    }
}

fn bail_usage() -> ! {
    eprintln!("\n{USAGE}");
    std::process::exit(2);
}

const USAGE: &str = "\
steam-puck-flasher -- flash Valve Steam Controller 2 puck (Proteus) firmware

usage:
  steam-puck-flasher              open the GUI (the normal way to use this)
  steam-puck-flasher --cli        run the same flow in a terminal
  steam-puck-flasher --help       this text
  steam-puck-flasher --version    print the version

environment:
  STEAM_PUCK_FLASHER_TRACE=1      log every bootloader frame to stderr
";

fn gui() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([880.0, 660.0])
            .with_min_inner_size([720.0, 520.0])
            .with_title("Steam Puck Flasher"),
        ..Default::default()
    };

    eframe::run_native(
        "Steam Puck Flasher",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()) as Box<dyn eframe::App>)),
    )
    .map_err(|e| anyhow::anyhow!("could not start the GUI: {e}"))
}
