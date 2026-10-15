use std::time::Duration;

use crate::flasher::{FlashProgress, FlashStage};
use eframe::egui;

use crate::app::App;
use crate::models::{Level, LogLine, Row, Step};
use crate::screens;
use crate::theme;
use crate::worker::Event;

impl App {
    pub(crate) fn pump(&mut self) {
        let Some(rx) = self.inbox.take() else {
            return;
        };
        while let Ok(event) = rx.try_recv() {
            self.handle(event);
        }
        self.inbox = Some(rx);
    }

    pub(crate) fn handle(&mut self, event: Event) {
        match event {
            Event::Catalogue(cat) => {
                if cat.index_generated_at.is_empty() {
                    self.log(
                        Level::Warn,
                        "the upstream index carries no generation timestamp, so channel \
                         membership may be stale",
                    );
                }
                self.rows = cat.entries.iter().map(Row::from_entry).collect();
                self.catalogue = Some(cat);
            }
            Event::CatalogueFailed(e) => {
                self.catalogue_error = e.clone();
                self.log(Level::Error, format!("catalogue unavailable: {e}"));
            }
            Event::Line(level, text) => self.log(level, text),
            Event::Progress(p) => self.absorb(p),
            Event::Finished(result) => {
                self.meter.fraction = 1.0;
                self.meter.headline = "Device reset and running the new image".into();
                self.log(
                    Level::Ok,
                    format!("{} is now running", result.firmware_name),
                );
                self.result = Some(format!(
                    "{} ({}) is now running on {}. {} KiB of payload written.",
                    result.firmware_name,
                    result.channel.tag(),
                    result.serial,
                    result.payload_size.div_ceil(1024),
                ));
                self.step = Step::Done;
            }
            Event::Failed(e) => {
                self.log(Level::Error, e.clone());
                self.failure = Some(e);
                self.step = Step::Done;
            }
        }
    }

    fn absorb(&mut self, p: FlashProgress) {
        let per_chunk = p.stage == FlashStage::Flashing;
        self.meter.apply(&p);

        if per_chunk || p.message.is_empty() {
            return;
        }
        if self.log.last().is_some_and(|l| l.text == p.message) {
            return;
        }
        let level = match p.stage {
            FlashStage::Resetting => Level::Ok,
            _ => Level::Info,
        };
        self.log.push(LogLine::new(level, p.message));
    }
}

/// Widest the content column is ever allowed to get.
const CONTENT_W: f32 = 760.0;

impl App {
    pub(crate) fn chrome(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(theme::BLACK)
                    .inner_margin(egui::Margin::symmetric(18.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.heading(
                        egui::RichText::new("Steam Controller 2 Puck Flasher")
                            .color(self.theme.text())
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("Step: {}", self.step.label()))
                                .color(self.theme.text())
                                .size(12.0),
                        );
                    });
                });
                ui.add_space(10.0);
                theme::rule(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(14.0);
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.set_max_width(CONTENT_W);
                screens::show(self, ui);
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump();
        theme::apply(ctx);
        self.chrome(ctx);
        ctx.request_repaint_after(Duration::from_millis(60));
    }
}
