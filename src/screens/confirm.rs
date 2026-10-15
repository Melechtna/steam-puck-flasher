//! Step 3 confirm before anything is written.

use eframe::egui;

use crate::app::App;
use crate::models::Selection;
use crate::theme::{self, Theme};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.theme;
    let Some(selection) = app.selection.clone() else {
        return;
    };

    let mut go = false;

    theme::card(theme, ui, |ui| {
        ui.heading(
            egui::RichText::new("Confirm flash")
                .color(theme.text())
                .size(20.0),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "The puck's application flash is erased and rewritten. Rolling back\n\
                 means flashing the stock image again.",
            )
            .color(theme::WARN)
            .size(13.0),
        );
        ui.add_space(18.0);

        ui.horizontal(|ui| {
            facts(ui, theme, &selection);
        });

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                go |= ui
                    .add(theme::action(
                        "Flash it",
                        theme::GO,
                        egui::vec2(140.0, 36.0),
                    ))
                    .clicked();
                ui.add_space(10.0);
                if ui
                    .add(theme::action("Back", theme::BACK, egui::vec2(100.0, 36.0)))
                    .clicked()
                {
                    app.back_to_catalogue();
                }
            });
        });
    });

    if go {
        app.begin_flash();
    }
}

fn facts(ui: &mut egui::Ui, theme: Theme, sel: &Selection) {
    let grid = egui::Grid::new("confirm")
        .num_columns(2)
        .spacing([26.0, 9.0]);
    grid.show(ui, |ui| {
        let mut row = |k: &str, v: &str, mono: bool, ink: egui::Color32| {
            ui.label(egui::RichText::new(k).color(theme.text()).size(12.5));
            let mut text = egui::RichText::new(v.to_string()).color(ink).size(12.5);
            if mono {
                text = text.monospace();
            }
            ui.label(text);
            ui.end_row();
        };

        row("Serial", &sel.serial, true, theme.text());
        row("Build", &sel.row.file_name, true, theme.text());
        row(
            "Channel",
            &format!("({})", sel.row.channel.tag()),
            false,
            badge_fill(sel),
        );
        row("First seen", sel.row.day(), true, theme.text());
        row("File size", &sel.row.size_h(), false, theme.text());
        row("CRC32", &sel.row.crc_h(), true, theme.text());
        row("SHA256", short(&sel.row.sha256), true, theme.text());
        row(
            "Verified",
            "on flash, against the two hashes above",
            false,
            theme.text(),
        );
    });
}

fn short(sha: &str) -> &str {
    match sha.get(..16) {
        Some(head) if sha.len() > 17 => head,
        _ => sha,
    }
}

fn badge_fill(sel: &Selection) -> egui::Color32 {
    use crate::flasher::catalog::Channel;
    match sel.row.channel {
        Channel::Stable => theme::STABLE,
        Channel::Beta => theme::BETA,
        Channel::User => theme::USER,
    }
}
