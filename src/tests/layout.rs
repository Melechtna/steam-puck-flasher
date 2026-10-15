//! Layout.

use egui::epaint::{ClippedPrimitive, Primitive};
use egui::{pos2, vec2, Color32, Context, Pos2, RawInput, Rect};

use crate::app::App;
use crate::theme::{self, JADE};

const DIVIDER_H: f32 = 3.0;

fn render(width: f32, height: f32, app: &mut App) -> Vec<ClippedPrimitive> {
    let ctx = Context::default();
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))),
        ..Default::default()
    };
    let mut last = Vec::new();
    for _ in 0..3 {
        let out = ctx.run(input.clone(), |ctx| {
            theme::apply(ctx);
            app.chrome(ctx);
        });
        last = ctx.tessellate(out.shapes, 1.0);
    }
    last
}

fn points_of(prims: &[ClippedPrimitive], colour: Color32) -> Vec<Pos2> {
    let mut points = Vec::new();
    for prim in prims {
        let Primitive::Mesh(mesh) = &prim.primitive else {
            continue;
        };
        points.extend(
            mesh.vertices
                .iter()
                .filter(|v| v.color == colour)
                .map(|v| v.pos),
        );
    }
    points
}

#[derive(Debug)]
struct Band {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

fn bands(points: &[Pos2], tolerance: f32) -> Vec<Band> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| a.y.total_cmp(&b.y));

    let mut out: Vec<Band> = Vec::new();
    for p in sorted {
        match out.last_mut() {
            Some(b) if p.y - b.top <= tolerance => {
                b.bottom = b.bottom.max(p.y);
                b.left = b.left.min(p.x);
                b.right = b.right.max(p.x);
            }
            _ => out.push(Band {
                top: p.y,
                bottom: p.y,
                left: p.x,
                right: p.x,
            }),
        }
    }
    out
}

fn dividers(prims: &[ClippedPrimitive], window_width: f32) -> Vec<Band> {
    bands(&points_of(prims, JADE), 2.0)
        .into_iter()
        .filter(|b| b.bottom - b.top <= DIVIDER_H && b.right - b.left > window_width * 0.9)
        .collect()
}

fn card_box(prims: &[ClippedPrimitive], window_width: f32) -> Rect {
    let cut = |p: &Pos2| {
        dividers(prims, window_width)
            .iter()
            .any(|b| p.y >= b.top && p.y <= b.bottom)
    };
    let inside: Vec<Pos2> = points_of(prims, JADE)
        .into_iter()
        .filter(|p| !cut(p))
        .collect();

    let mut r = Rect::from_min_max(inside[0], inside[0]);
    for p in inside {
        r.extend_with(p);
    }
    r
}

fn offline() -> App {
    let mut app = App::bare();
    app.detected.clear();
    app
}

#[test]
fn the_card_is_centred_in_a_wide_window() {
    let (w, h) = (1400.0, 900.0);
    let card = card_box(&render(w, h, &mut offline()), w);

    assert!(
        (card.center().x - w / 2.0).abs() < 1.0,
        "card centre at x={} but the window centre is {} -- it is still packed left",
        card.center().x,
        w / 2.0
    );
}

#[test]
fn the_card_holds_a_fixed_width_rather_than_following_its_contents() {
    let (w, h) = (1400.0, 900.0);
    let card = card_box(&render(w, h, &mut offline()), w);

    assert!(
        (card.width() - 760.0).abs() < 2.0,
        "card is {} wide, expected the 760 cap",
        card.width()
    );
}

#[test]
fn the_card_still_fits_a_narrow_window() {
    let (w, h) = (420.0, 800.0);
    let card = card_box(&render(w, h, &mut offline()), w);

    assert!(
        card.max.x <= w + 1.0 && card.min.x >= -1.0,
        "card at x={}..{} overflows a {w} wide window",
        card.min.x,
        card.max.x
    );
    assert!(
        (card.center().x - w / 2.0).abs() < 1.0,
        "card centre at x={} but the window centre is {}",
        card.center().x,
        w / 2.0
    );
}

#[test]
fn exactly_one_line_divides_the_title_bar_from_the_page() {
    let (w, h) = (1400.0, 900.0);
    let found = dividers(&render(w, h, &mut offline()), w);

    assert_eq!(
        found.len(),
        1,
        "expected a single jade divider under the title bar, found {found:#?}"
    );
    assert!(
        found[0].top < h * 0.15,
        "the divider at y={} is not up in the title bar",
        found[0].top
    );
}
