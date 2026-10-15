//! Step 4:, the flash in progress.

use eframe::egui;

use crate::app::App;
use crate::models::{Level, LogLine};
use crate::theme::{self, Theme};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.theme;
    let App { meter, log, .. } = app;

    theme::card(theme, ui, |ui| {
        ui.heading(
            egui::RichText::new("Flashing")
                .color(theme.text())
                .size(20.0),
        );
        ui.add_space(10.0);

        ui.add(
            egui::ProgressBar::new(meter.fraction)
                .fill(theme::ACCENT)
                .desired_width(ui.available_width())
                .show_percentage(),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(meter.headline.as_str())
                    .color(theme.text())
                    .size(12.5),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(line) = meter.chunk_line() {
                    ui.label(
                        egui::RichText::new(line)
                            .color(theme.text())
                            .monospace()
                            .size(11.5),
                    );
                }
            });
        });

        ui.add_space(14.0);
        theme::well(theme, ui, |ui| {
            ui.set_min_height(220.0);
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if log.is_empty() {
                        ui.label(egui::RichText::new("waiting...").color(theme.text()));
                    }
                    for line in log {
                        entry(ui, theme, line);
                    }
                });
        });
    });
}

fn entry(ui: &mut egui::Ui, theme: Theme, line: &LogLine) {
    let ink = match line.level {
        Level::Info => theme.text(),
        Level::Warn => theme::WARN,
        Level::Error => theme::ERR,
        Level::Ok => theme::OK,
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(line.at.as_str())
                .color(theme.text())
                .monospace()
                .size(10.5),
        );
        ui.allocate_space(egui::vec2(8.0, 0.0));
        ui.painter()
            .circle_filled(ui.cursor().left_top() + egui::vec2(3.5, 8.0), 3.0, ink);
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(line.text.as_str())
                .color(ink)
                .monospace()
                .size(11.5),
        );
    });
}
