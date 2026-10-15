//! Application state and the transitions between steps.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::flasher::catalog;

use crate::models::{Level, LogLine, Meter, Row, Selection, Step};
use crate::theme::Theme;
use crate::worker::{self, Event};

pub struct App {
    pub step: Step,
    pub theme: Theme,

    pub serial_text: String,
    pub serial_error: String,
    pub detected: Vec<String>,

    pub catalogue: Option<Arc<catalog::Catalog>>,
    pub rows: Vec<Row>,
    pub catalogue_error: String,
    pub selected: Option<usize>,

    pub selection: Option<Selection>,

    pub meter: Meter,
    pub log: Vec<LogLine>,
    pub result: Option<String>,
    pub failure: Option<String>,

    pub(crate) inbox: Option<Receiver<Event>>,
    pub(crate) outbox: Sender<Event>,
}

impl App {
    pub fn bare() -> Self {
        let (tx, rx) = channel();

        Self {
            step: Step::Serial,
            theme: Theme::new(),
            serial_text: String::new(),
            serial_error: String::new(),
            detected: worker::detect_serials(),
            catalogue: None,
            rows: Vec::new(),
            catalogue_error: String::new(),
            selected: None,
            selection: None,
            meter: Meter::default(),
            log: Vec::new(),
            result: None,
            failure: None,
            inbox: Some(rx),
            outbox: tx,
        }
    }

    pub fn new() -> Self {
        let app = Self::bare();
        worker::spawn_catalogue(app.outbox.clone());
        app
    }

    pub fn log(&mut self, level: Level, text: impl Into<String>) {
        self.log.push(LogLine::new(level, text));
    }

    pub fn accept_serial(&mut self) {
        self.serial_error.clear();
        let candidate = self.serial_text.trim().to_uppercase();

        if candidate.is_empty() {
            self.serial_error = "a serial number is required".into();
        } else if let Some(bad) = candidate.chars().find(|c| !c.is_ascii_alphanumeric()) {
            self.serial_error = format!("'{bad}' is not allowed -- letters and digits only");
        } else if candidate.len() > 20 {
            self.serial_error = format!(
                "that is {} characters; a serial is at most 20",
                candidate.len()
            );
        } else {
            self.serial_text = candidate;
            self.log(Level::Info, format!("Serial: {}", self.serial_text));
            self.step = Step::Catalogue;
        }
    }

    /// Put a serial detected on the bus into the field.
    pub fn use_detected_serial(&mut self, found: &str) {
        self.serial_text.clear();
        self.serial_text.push_str(found);
        self.serial_error.clear();
    }

    pub fn retry_catalogue(&mut self) {
        self.catalogue_error.clear();
        self.catalogue = None;
        worker::spawn_catalogue(self.outbox.clone());
    }

    /// Check or uncheck a build.
    pub fn toggle_selection(&mut self, index: usize) {
        if index >= self.rows.len() {
            return;
        }
        self.selected = if self.selected == Some(index) {
            None
        } else {
            Some(index)
        };
        self.selection = None;
    }

    pub fn choose(&mut self, index: usize) {
        let Some(row) = self.rows.get(index).cloned() else {
            return;
        };
        self.selected = Some(index);
        self.selection = Some(Selection {
            serial: self.serial_text.clone(),
            row,
        });
        self.step = Step::Confirm;
    }

    pub fn back_to_catalogue(&mut self) {
        self.step = Step::Catalogue;
    }

    pub fn back_to_serial(&mut self) {
        self.step = Step::Serial;
    }

    pub fn begin_flash(&mut self) {
        let (Some(cat), Some(sel), Some(index)) = (
            self.catalogue.clone(),
            self.selection.clone(),
            self.selected,
        ) else {
            self.log(Level::Error, "internal error: nothing selected to flash");
            return;
        };

        self.log.clear();
        self.meter = Meter::default();
        self.result = None;
        self.failure = None;
        self.step = Step::Flashing;
        self.log(
            Level::Info,
            format!("Flashing {} to {}", sel.row.file_name, sel.serial),
        );

        worker::spawn_flash(self.outbox.clone(), cat, index, sel.serial);
    }
}
