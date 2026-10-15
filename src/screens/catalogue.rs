//! Step 2, the firmware catalogue.

use eframe::egui;

use crate::app::App;
use crate::models::Row;
use crate::theme::{self, Theme};

const DATE_W: f32 = 96.0;
const BADGE_W: f32 = 88.0;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.theme;
    let App {
        rows,
        selected,
        catalogue_error,
        ..
    } = app;

    let loading = rows.is_empty() && catalogue_error.is_empty();

    let mut tick: Option<usize> = None;
    let mut go = false;
    let mut retry = false;
    let mut back = false;

    theme::card(theme, ui, |ui| {
        ui.heading(
            egui::RichText::new("Choose a build")
                .color(theme.text())
                .size(20.0),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Channel tags come from the Ibex-Firmware index; (user) builds are read\n\
                 from the firmware folder beside this executable.",
            )
            .color(theme.text()),
        );
        ui.add_space(14.0);

        if loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    egui::RichText::new("Reading the firmware catalogue...").color(theme.text()),
                );
            });
        } else if !catalogue_error.is_empty() {
            ui.label(egui::RichText::new(catalogue_error.as_str()).color(theme::ERR));
            ui.add_space(8.0);
            if ui
                .add(theme::action(
                    "Try again",
                    theme::FORWARD,
                    egui::vec2(140.0, 32.0),
                ))
                .clicked()
            {
                retry = true;
            }
        } else {
            header(ui, theme);

            egui::ScrollArea::vertical()
                .max_height(360.0)
                .auto_shrink([false, false])
                .stick_to_bottom(false)
                .show(ui, |ui| {
                    for (i, entry) in rows.iter().enumerate() {
                        if row(ui, theme, entry, *selected == Some(i), i) {
                            tick = Some(i);
                        }
                    }
                });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} builds", rows.len())).color(theme.text()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pressed = ui
                        .add_enabled(
                            selected.is_some(),
                            theme::action("Continue", theme::FORWARD, egui::vec2(130.0, 32.0)),
                        )
                        .clicked();
                    if pressed {
                        go = true;
                    }
                    ui.add_space(10.0);
                    if ui
                        .add(theme::action("Back", theme::BACK, egui::vec2(100.0, 32.0)))
                        .clicked()
                    {
                        back = true;
                    }
                });
            });
        }
    });

    if go {
        if let Some(index) = *selected {
            app.choose(index);
        }
    } else if retry {
        app.retry_catalogue();
    } else if back {
        app.back_to_serial();
    } else if let Some(index) = tick {
        app.toggle_selection(index);
    }
}

fn header(ui: &mut egui::Ui, theme: Theme) {
    ui.horizontal(|ui| {
        let h = |ui: &mut egui::Ui, w: f32, s: &str| {
            ui.label(
                egui::RichText::new(s)
                    .color(theme.text())
                    .size(11.0)
                    .strong(),
            );
            ui.allocate_space(egui::vec2(w, ui.spacing().interact_size.y));
        };
        h(ui, DATE_W, "FIRST SEEN");
        h(ui, BADGE_W, "CHANNEL");
        h(ui, 0.0, "BUILD");
        h(ui, 84.0, "SIZE");
    });
    ui.add_space(4.0);
}

/// Returns true if the user picked this build.
fn row(ui: &mut egui::Ui, theme: Theme, entry: &Row, selected: bool, index: usize) -> bool {
    let bg = if selected {
        theme::ACCENT.gamma_multiply(0.38)
    } else {
        theme::BLACK
    };

    let mut picked = false;

    let inner = egui::Frame::none()
        .fill(bg)
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut on = selected;
                if ui.checkbox(&mut on, "").clicked() {
                    picked = true;
                }

                ui.allocate_space(egui::vec2(DATE_W, 18.0));
                ui.label(
                    egui::RichText::new(entry.day())
                        .color(theme.text())
                        .monospace()
                        .size(11.5),
                );

                theme::badge(
                    theme,
                    ui,
                    &format!("({})", entry.channel.tag()),
                    channel_fill(entry),
                    BADGE_W,
                );

                ui.label(
                    egui::RichText::new(entry.file_name.as_str())
                        .color(theme.text())
                        .monospace()
                        .size(12.5),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(entry.size_h())
                            .color(theme.text())
                            .size(11.5),
                    );
                    ui.allocate_space(egui::vec2(4.0, 0.0));
                });
            });
        });

    let hit = ui.interact(
        inner.response.rect.expand(2.0),
        ui.id().with(("row", index)),
        egui::Sense::click(),
    );
    picked |= hit.clicked();

    ui.add_space(3.0);
    picked
}

fn channel_fill(entry: &Row) -> egui::Color32 {
    use crate::flasher::catalog::Channel;
    match entry.channel {
        Channel::Stable => theme::STABLE,
        Channel::Beta => theme::BETA,
        Channel::User => theme::USER,
    }
}
