//! Step 5, outcome

use eframe::egui;

use crate::app::App;
use crate::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.theme;
    let App {
        result, failure, ..
    } = app;

    let ok = failure.is_none();
    let mut back = false;

    theme::card(theme, ui, |ui| {
        ui.vertical_centered(|ui| {
            let (mark, head, ink) = if ok {
                ("\u{2713}", "Flash complete", theme::OK)
            } else {
                ("\u{2717}", "Flash failed", theme::ERR)
            };

            ui.add_space(10.0);
            ui.heading(
                egui::RichText::new(format!("{mark} {head}"))
                    .color(ink)
                    .size(26.0),
            );
            ui.add_space(12.0);

            if let Some(text) = result {
                ui.label(
                    egui::RichText::new(text.as_str())
                        .color(theme.text())
                        .size(14.0),
                );
            }

            if let Some(why) = failure {
                ui.label(
                    egui::RichText::new("Nothing was left half-written; the puck is unchanged.")
                        .color(theme.text()),
                );
                ui.add_space(10.0);
                theme::well(theme, ui, |ui| {
                    ui.set_min_width(460.0);
                    ui.label(
                        egui::RichText::new(why.as_str())
                            .color(theme::ERR)
                            .monospace()
                            .size(12.5),
                    );
                });
            }

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui
                    .add(theme::action(
                        if ok { "Close" } else { "Back to catalogue" },
                        theme::BACK,
                        egui::vec2(160.0, 36.0),
                    ))
                    .clicked()
                {
                    if ok {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        back = true;
                    }
                }
            });
        });
    });

    if back {
        app.back_to_catalogue();
    }
}
