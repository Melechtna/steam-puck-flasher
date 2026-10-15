//! Step 1, the serial number.

use eframe::egui;

use crate::app::App;
use crate::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let theme = app.theme;
    let App {
        serial_text,
        serial_error,
        detected,
        ..
    } = app;

    let mut submit = false;
    let mut adopt: Option<String> = None;

    theme::card(theme, ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading(
                egui::RichText::new("Enter the puck's serial number")
                    .color(theme.text())
                    .size(20.0),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "The serial is printed on the puck and shown in Steam.\n\
                     It is what gets written into the provisioning block.",
                )
                .color(theme.text()),
            );
            ui.add_space(16.0);

            if !detected.is_empty() {
                ui.label(
                    egui::RichText::new("Detected on the bus")
                        .color(theme::OK)
                        .size(13.0)
                        .strong(),
                );
                ui.add_space(6.0);
                for found in detected.iter() {
                    ui.horizontal(|ui| {
                        ui.add_space(48.0);
                        ui.label(
                            egui::RichText::new(found.as_str())
                                .color(theme::OK)
                                .monospace()
                                .size(14.0),
                        );
                        if ui.small_button("Use").clicked() {
                            adopt = Some(found.clone());
                        }
                    });
                }
                ui.add_space(14.0);
            }

            let ready = serial_text.trim().len() >= 3;
            let field = ui.add(
                egui::TextEdit::singleline(serial_text)
                    .font(egui::FontId::monospace(16.0))
                    .hint_text("Serial #")
                    .desired_width(320.0),
            );
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }

            if !serial_error.is_empty() {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(serial_error.as_str()).color(theme::ERR));
            }

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let go = ui
                        .add_enabled(
                            ready,
                            theme::action("Continue", theme::FORWARD, egui::vec2(130.0, 34.0)),
                        )
                        .clicked();
                    submit |= go;
                    ui.add_space(10.0);
                    if ui
                        .add(theme::action("Quit", theme::BACK, egui::vec2(100.0, 34.0)))
                        .clicked()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });
    });

    if let Some(found) = adopt {
        app.use_detected_serial(&found);
    }
    if submit {
        app.accept_serial();
    }
}
