//! Colours and style.

use egui::{self, Color32, Context, FontFamily, FontId, TextStyle};

pub const ACCENT: Color32 = Color32::from_rgb(0x7b, 0x2d, 0xbe);
pub const JADE: Color32 = Color32::from_rgb(0x00, 0xa8, 0x6b);
pub const BLACK: Color32 = Color32::BLACK;
pub const STABLE: Color32 = Color32::from_rgb(0x00, 0xc8, 0x53);
pub const BETA: Color32 = Color32::from_rgb(0x00, 0xb4, 0xd8);
pub const USER: Color32 = ACCENT;
pub const OK: Color32 = Color32::from_rgb(0x00, 0xc8, 0x53);
pub const WARN: Color32 = Color32::from_rgb(0xff, 0xd6, 0x00);
pub const ERR: Color32 = Color32::from_rgb(0xff, 0x17, 0x44);
pub const FORWARD: Color32 = Color32::from_rgb(0x00, 0xb4, 0xd8);
pub const BACK: Color32 = Color32::from_rgb(0xdc, 0x14, 0x3c);
pub const GO: Color32 = Color32::from_rgb(0x7e, 0xe7, 0x87);

/// The palette.
#[derive(Clone, Copy)]
pub struct Theme;

impl Theme {
    pub fn new() -> Self {
        Self
    }

    pub fn text(&self) -> Color32 {
        ACCENT
    }

    pub fn surface(&self) -> Color32 {
        BLACK
    }

    pub fn well(&self) -> Color32 {
        BLACK
    }

    pub fn stroke(&self) -> egui::Stroke {
        egui::Stroke::new(1.0, JADE)
    }

    pub fn badge_ink(&self, fill: Color32) -> Color32 {
        if fill == USER {
            Color32::WHITE
        } else {
            Color32::BLACK
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

/// Push the whole look.
pub fn apply(ctx: &Context) {
    let mut style = (*ctx.style()).clone();

    let mut v = egui::Visuals::dark();

    v.panel_fill = BLACK;
    v.window_fill = BLACK;
    v.extreme_bg_color = BLACK;
    v.faint_bg_color = BLACK;
    v.override_text_color = Some(ACCENT);
    v.window_stroke = egui::Stroke::new(1.0, JADE);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, JADE);
    v.widgets.inactive.bg_fill = BLACK;
    v.widgets.inactive.weak_bg_fill = BLACK;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, JADE);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.hovered.bg_fill = BLACK;
    v.widgets.hovered.weak_bg_fill = BLACK;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, JADE.gamma_multiply(1.35));
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ACCENT);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, JADE);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::WHITE);
    v.widgets.open.bg_fill = BLACK;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, JADE);
    v.selection.bg_fill = ACCENT.gamma_multiply(1.6);
    v.selection.stroke = egui::Stroke::new(1.0, JADE);
    v.hyperlink_color = ACCENT;
    v.error_fg_color = ERR;
    v.warn_fg_color = WARN;

    style.visuals = v;
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(21.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    let rounding = egui::Rounding::same(6.0);
    for w in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
        &mut style.visuals.widgets.noninteractive,
    ] {
        w.rounding = rounding;
    }

    ctx.set_style(style);
}

/// A titled card.
pub fn card(theme: Theme, ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(theme.surface())
        .stroke(theme.stroke())
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::same(22.0))
        .show(ui, add);
}

/// Log pane.
pub fn well(theme: Theme, ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(theme.well())
        .stroke(theme.stroke())
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, add);
}

/// Used for the line under the title bar.
pub fn rule(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_at_least(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, JADE);
}

/// A filled action button.
pub fn action(label: &str, fill: Color32, size: egui::Vec2) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(label).color(BLACK).strong())
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, fill.gamma_multiply(1.3)))
        .min_size(size)
}

/// A rounded pill.
pub fn badge(theme: Theme, ui: &mut egui::Ui, text: &str, fill: Color32, width: f32) {
    let (rect, _) = ui.allocate_at_least(egui::vec2(width, 22.0), egui::Sense::hover());
    let rounding = egui::Rounding::same(11.0);
    ui.painter().rect_filled(rect, rounding, fill);
    ui.painter().rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0, fill.gamma_multiply(1.25)),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(11.5),
        theme.badge_ink(fill),
    );
}
