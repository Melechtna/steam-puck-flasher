//! The palette.

use egui::Context;

use crate::theme::{self, Theme, ACCENT, BACK, BLACK, FORWARD, GO, JADE};

fn luminance(c: egui::Color32) -> f32 {
    fn channel(v: u8) -> f32 {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
}

fn contrast(a: egui::Color32, b: egui::Color32) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

fn applied() -> egui::Visuals {
    let ctx = Context::default();
    let mut out = None;
    let _ = ctx.run(Default::default(), |ctx| {
        theme::apply(ctx);
        out = Some(ctx.style().visuals.clone());
    });
    out.expect("context should have run at least once")
}

#[test]
fn accent_is_royal_purple() {
    assert_eq!([ACCENT.r(), ACCENT.g(), ACCENT.b()], [0x7b, 0x2d, 0xbe]);
}

#[test]
fn jade_is_jade() {
    assert_eq!([JADE.r(), JADE.g(), JADE.b()], [0x00, 0xa8, 0x6b]);
}

#[test]
fn the_background_is_pure_black_not_near_black() {
    assert_eq!([BLACK.r(), BLACK.g(), BLACK.b()], [0, 0, 0]);
}

#[test]
fn every_background_egui_fills_is_pure_black() {
    let v = applied();
    for (name, got) in [
        ("panel_fill", v.panel_fill),
        ("window_fill", v.window_fill),
        ("extreme_bg_color", v.extreme_bg_color),
        ("faint_bg_color", v.faint_bg_color),
    ] {
        assert_eq!(
            [got.r(), got.g(), got.b()],
            [0, 0, 0],
            "{name} is {got:?}, expected pure black"
        );
    }
}

#[test]
fn every_widget_fill_is_pure_black_until_pressed() {
    let v = applied();
    let widgets = [
        ("inactive", v.widgets.inactive),
        ("inactive.weak", v.widgets.inactive),
        ("hovered", v.widgets.hovered),
        ("open", v.widgets.open),
    ];
    for (name, w) in widgets {
        assert_eq!(
            [w.bg_fill.r(), w.bg_fill.g(), w.bg_fill.b()],
            [0, 0, 0],
            "{name} bg_fill is {:?}, expected pure black",
            w.bg_fill
        );
    }
    assert_eq!(v.widgets.active.bg_fill, ACCENT);
}

#[test]
fn the_cards_and_wells_are_black_too() {
    let t = Theme::new();
    assert_eq!(
        [t.surface().r(), t.surface().g(), t.surface().b()],
        [0, 0, 0]
    );
    assert_eq!([t.well().r(), t.well().g(), t.well().b()], [0, 0, 0]);
}

#[test]
fn text_defaults_to_royal_purple() {
    assert_eq!(applied().override_text_color, Some(ACCENT));
}

#[test]
fn there_is_only_one_text_brightness() {
    let t = Theme::new();
    assert_eq!(t.text(), ACCENT, "body text should be the accent itself");
}

// ---- the requirement: jade borders ----------------------------------------

#[test]
fn window_and_separator_strokes_are_jade() {
    let v = applied();
    assert_eq!(v.window_stroke.color, JADE);
    assert_eq!(v.widgets.noninteractive.bg_stroke.color, JADE);
}

#[test]
fn the_card_border_is_jade() {
    assert_eq!(Theme::new().stroke().color, JADE);
}

#[test]
fn body_text_contrast_against_black_is_measured() {
    let ratio = contrast(ACCENT, BLACK);
    assert!(
        ratio > 1.0,
        "sanity: some contrast must exist, got {ratio:.2}"
    );
    println!("body text contrast on black: {ratio:.2}:1");
    println!(
        "jade border contrast on black: {:.2}:1",
        contrast(JADE, BLACK)
    );
}

#[test]
fn forward_buttons_are_baby_blue() {
    assert_eq!([FORWARD.r(), FORWARD.g(), FORWARD.b()], [0x00, 0xb4, 0xd8]);
}

#[test]
fn back_and_quit_buttons_are_crimson() {
    assert_eq!([BACK.r(), BACK.g(), BACK.b()], [0xdc, 0x14, 0x3c]);
}

#[test]
fn the_flash_button_is_light_green() {
    assert_eq!([GO.r(), GO.g(), GO.b()], [0x7e, 0xe7, 0x87]);
}

#[test]
fn every_button_fill_carries_black_ink_legibly() {
    for (name, fill) in [("FORWARD", FORWARD), ("BACK", BACK), ("GO", GO)] {
        let ratio = contrast(fill, BLACK);
        println!("black on {name}: {ratio:.2}:1");
        assert!(
            ratio >= 4.0,
            "black text on {name} is only {ratio:.2}:1 -- the label will not read"
        );
    }
}

#[test]
fn the_three_button_fills_are_distinguishable() {
    let fills = [FORWARD, BACK, GO];
    for (i, a) in fills.iter().enumerate() {
        for b in &fills[i + 1..] {
            let distance = (a.r() as i32 - b.r() as i32).abs()
                + (a.g() as i32 - b.g() as i32).abs()
                + (a.b() as i32 - b.b() as i32).abs();
            assert!(distance > 120, "{a:?} and {b:?} are too close: {distance}");
        }
    }
}
