//! One module per screen.

pub mod catalogue;
pub mod confirm;
pub mod done;
pub mod flashing;
pub mod serial;

use eframe::egui;

use crate::app::App;
use crate::models::Step;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    match app.step {
        Step::Serial => serial::show(app, ui),
        Step::Catalogue => catalogue::show(app, ui),
        Step::Confirm => confirm::show(app, ui),
        Step::Flashing => flashing::show(app, ui),
        Step::Done => done::show(app, ui),
    }
}
