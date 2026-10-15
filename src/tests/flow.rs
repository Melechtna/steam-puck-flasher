//! Every serial that is rejected must say why..

use crate::app::App;
use crate::flasher::catalog::Channel;
use crate::models::{Level, Row, Step};

fn row() -> Row {
    Row {
        file_name: "STABLE_0001.fw".into(),
        channel: Channel::Stable,
        date: "2024-03-01T00:00:00Z".into(),
        size: 4096,
        crc32: 0xABCD_1234,
        sha256: "c".repeat(64),
    }
}

fn row_named(name: &str) -> Row {
    Row {
        file_name: name.into(),
        ..row()
    }
}

/// An app sitting on the catalogue.
fn browsing() -> App {
    let mut app = App::bare();
    app.serial_text = "FXB1".into();
    app.rows = vec![row_named("A.fw"), row_named("B.fw"), row_named("C.fw")];
    app.accept_serial();
    app
}

#[test]
fn a_good_serial_advances_to_the_catalogue() {
    let mut app = App::bare();
    app.serial_text = " fxb996010210b ".into();
    app.accept_serial();

    assert_eq!(app.step, Step::Catalogue);
    assert_eq!(
        app.serial_text, "FXB996010210B",
        "serials are trimmed and upcased"
    );
    assert!(app.serial_error.is_empty());
    assert!(app.log.iter().any(|l| l.text.contains("FXB996010210B")));
}

#[test]
fn an_empty_serial_is_refused() {
    let mut app = App::bare();
    app.accept_serial();

    assert_eq!(app.step, Step::Serial);
    assert_eq!(app.serial_error, "a serial number is required");
}

#[test]
fn a_serial_with_punctuation_names_the_offending_character() {
    let mut app = App::bare();
    app.serial_text = "FXB-996".into();
    app.accept_serial();

    assert_eq!(app.step, Step::Serial);
    assert!(app.serial_error.contains("'-'"), "got {}", app.serial_error);
}

#[test]
fn an_overlong_serial_says_how_long_it_was() {
    let mut app = App::bare();
    app.serial_text = "A".repeat(21);
    app.accept_serial();

    assert_eq!(app.step, Step::Serial);
    assert!(
        app.serial_error.contains("21 characters"),
        "got {}",
        app.serial_error
    );
}

#[test]
fn a_rejection_does_not_leave_the_old_error_behind() {
    let mut app = App::bare();
    app.accept_serial();
    assert!(!app.serial_error.is_empty());

    app.serial_text = "FXB1".into();
    app.accept_serial();
    assert!(app.serial_error.is_empty());
}

#[test]
fn choosing_a_row_freezes_the_selection_for_the_confirm_step() {
    let mut app = App::bare();
    app.serial_text = "FXB1".into();
    app.rows = vec![row()];
    app.accept_serial();
    app.choose(0);

    assert_eq!(app.step, Step::Confirm);
    let sel = app.selection.as_ref().unwrap();
    assert_eq!(sel.serial, "FXB1");
    assert_eq!(sel.row.file_name, "STABLE_0001.fw");
    assert_eq!(app.selected, Some(0));
}

#[test]
fn choosing_a_row_that_does_not_exist_is_a_no_op() {
    let mut app = App::bare();
    app.choose(7);

    assert_eq!(app.step, Step::Serial);
    assert!(app.selection.is_none());
    assert!(app.selected.is_none());
}

#[test]
fn ticking_a_build_checks_it_without_advancing() {
    let mut app = browsing();
    assert_eq!(app.step, Step::Catalogue);

    app.toggle_selection(1);

    assert_eq!(app.selected, Some(1), "the box should now be ticked");
    assert_eq!(app.step, Step::Catalogue, "but we should still be browsing");
    assert!(app.selection.is_none(), "nothing should be frozen yet");
}

#[test]
fn ticking_the_same_build_again_unticks_it() {
    let mut app = browsing();
    app.toggle_selection(0);
    app.toggle_selection(0);

    assert_eq!(app.selected, None, "a second tick should clear the box");
    assert_eq!(app.step, Step::Catalogue);
}

#[test]
fn only_one_build_can_be_ticked_at_a_time() {
    let mut app = browsing();
    app.toggle_selection(0);
    app.toggle_selection(2);

    assert_eq!(app.selected, Some(2), "the newer tick wins");
}

#[test]
fn ticking_a_build_that_does_not_exist_is_a_no_op() {
    let mut app = browsing();
    app.toggle_selection(0);
    app.toggle_selection(99);

    assert_eq!(app.selected, Some(0), "an out-of-range tick must not clear");
}

#[test]
fn ticking_a_build_discards_a_previously_frozen_choice() {
    // The confirm screen holds a snapshot. Once the user goes back and ticks
    // something else, that snapshot is no longer what they chose and must not be
    // flashable as-is.
    let mut app = browsing();
    app.choose(0);
    assert_eq!(app.step, Step::Confirm);

    app.back_to_catalogue();
    app.toggle_selection(2);

    assert!(app.selection.is_none(), "the stale snapshot should be gone");
    assert_eq!(app.selected, Some(2));
}

#[test]
fn continuing_freezes_the_ticked_build_and_advances() {
    let mut app = browsing();
    app.toggle_selection(2);
    app.choose(app.selected.expect("a build is ticked"));

    assert_eq!(app.step, Step::Confirm);
    assert_eq!(app.selection.as_ref().unwrap().row.file_name, "C.fw");
}

#[test]
fn using_a_detected_serial_only_fills_the_field() {
    let mut app = App::bare();
    app.use_detected_serial("FXB996010210B");

    assert_eq!(app.serial_text, "FXB996010210B");
    assert_eq!(app.step, Step::Serial, "still on the serial page");
}

#[test]
fn using_a_detected_serial_replaces_whatever_was_typed() {
    let mut app = App::bare();
    app.serial_text = "SOMETHING".into();
    app.use_detected_serial("FXB996010210B");

    assert_eq!(app.serial_text, "FXB996010210B");
}

#[test]
fn using_a_detected_serial_clears_a_stale_error() {
    let mut app = App::bare();
    app.accept_serial();
    assert!(!app.serial_error.is_empty());

    app.use_detected_serial("FXB996010210B");
    assert!(app.serial_error.is_empty());
}

#[test]
fn a_detected_serial_still_has_to_be_accepted_explicitly() {
    let mut app = App::bare();
    app.use_detected_serial("FXB996010210B");
    assert_eq!(app.step, Step::Serial);

    app.accept_serial();
    assert_eq!(app.step, Step::Catalogue);
}

#[test]
fn back_buttons_return_to_the_previous_step() {
    let mut app = App::bare();
    app.step = Step::Confirm;
    app.back_to_catalogue();
    assert_eq!(app.step, Step::Catalogue);
    app.back_to_serial();
    assert_eq!(app.step, Step::Serial);
}

#[test]
fn logging_records_every_line() {
    let mut app = App::bare();
    app.log(Level::Info, "one");
    app.log(Level::Error, "two");

    assert_eq!(app.log.len(), 2);
    assert_eq!(app.log[1].level, Level::Error);
    assert_eq!(app.log[1].text, "two");
    assert!(!app.log[0].at.is_empty(), "lines are timestamped");
}
