//! Where every part of the settings page goes, in physical pixels.
//!
//! The constants are logical units (pixels at 96 DPI), multiplied by the
//! window's scale (`GetDpiForWindow / 96`). Text is measured with the real
//! font, through the `measure` function the window passes in, so labels,
//! check boxes and wrapped notes fit whatever message font and text size the
//! user runs, and the window grows instead of clipping. The page reserves
//! room for the longest mode footer and hint, so it never changes size while
//! it is open.
//!
//! ```text
//!  MARGIN
//!  | Mode         (•) On Schedule  ( ) Always On  ( ) Off
//!  |              <mode footer>
//!  | Schedule
//!  |   Days       [x]Sun [x]Mon ...
//!  |   From       [09:00]
//!  |   To         [18:00]
//!  |              <hint>
//!  | Movement
//!  |   Distance   [Gentle · up to 5 pt v]
//!  |   ...
//!  |   Try the movement now  [Preview]
//!  |   <preview note, the full width>
//!  |   <movement footer, the full width>
//!  | General
//!  |   [x] Launch at login
//!  |                                        [Close]
//!    ^ label_x    ^ control_x
//! ```

use melcat_core::{JiggleMode, Weekday};

use super::page::{Part, MODE_FOOTERS, MOVEMENT_FOOTER, PREVIEW_NOTE};
use crate::geom::RectPx;

/// Space around the page.
const MARGIN: i32 = 16;
const TOP: i32 = 12;
const BOTTOM: i32 = 14;
/// Rows under a section header are indented by this much.
const INDENT: i32 = 12;
/// Between the label column and the control column.
const COLUMN_GAP: i32 = 16;
/// A control row is at least this tall, and at least a line of text plus `ROW_PAD`.
const ROW: i32 = 24;
const ROW_PAD: i32 = 8;
const ROW_GAP: i32 = 6;
/// Between a row and the note under it.
const TEXT_GAP: i32 = 4;
/// Before a section header, and between a header and its first row.
const SECTION_GAP: i32 = 16;
const HEADER_GAP: i32 = 6;
/// A check box or radio button is its text plus the box and the gap after it.
const CHECK_EXTRA: i32 = 22;
const RADIO_GAP: i32 = 12;
const DAY_GAP: i32 = 2;
/// The control column is never narrower than this.
const CONTROL_MIN: i32 = 320;
/// A time picker is its text plus the up-down buttons and padding.
const PICKER_MIN: i32 = 112;
const PICKER_EXTRA: i32 = 44;
/// A drop-down list is its longest entry plus the arrow and padding.
const COMBO_MIN: i32 = 208;
const COMBO_EXTRA: i32 = 40;
/// A push button is its text plus padding.
const BUTTON_MIN: i32 = 88;
const BUTTON_PAD: i32 = 24;
/// Above the Close button.
const CLOSE_GAP: i32 = 18;

/// The two fonts of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Face {
    /// The system message font.
    Body,
    /// The same, semibold, for the section headers.
    Header,
}

/// Measures `text` in `face`, in physical pixels: `(width, height)` of one
/// line when `wrap` is `None`, or of the text word-wrapped to `wrap` pixels.
pub(super) type Measure<'a> = &'a dyn Fn(Face, &str, Option<i32>) -> (i32, i32);

/// The texts whose size the layout cannot know in advance: they depend on
/// the locale and on the settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Texts {
    /// Every entry of the two drop-down lists.
    pub(super) choices: Vec<String>,
    /// A morning and an evening time in the user's format, as the pickers show them.
    pub(super) times: Vec<String>,
    /// Every hint the schedule can produce (`page::hint_samples`).
    pub(super) hints: Vec<String>,
}

/// The page at one scale: the client size and a rectangle for every part,
/// in client coordinates.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Layout {
    pub(super) width: i32,
    pub(super) height: i32,
    /// One entry per part, in `Part::ORDER`.
    pub(super) parts: Vec<(Part, RectPx)>,
}

impl Layout {
    /// The rectangle of `part` (empty if the layout has none, which never happens).
    #[cfg(test)]
    pub(super) fn rect(&self, part: Part) -> RectPx {
        self.parts.iter().find(|(p, _)| *p == part).map(|&(_, rect)| rect).unwrap_or_default()
    }
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> RectPx {
    RectPx::new(x, y, x + width, y + height)
}

fn widest(values: impl IntoIterator<Item = i32>) -> i32 {
    values.into_iter().max().unwrap_or(0)
}

/// The top-left corner that centers a `width` x `height` window in `area`
/// (a monitor's work area), or puts it at the area's top-left corner when it
/// does not fit, so the caption stays reachable.
pub(super) fn centered(area: RectPx, width: i32, height: i32) -> (i32, i32) {
    (area.left + ((area.width() - width) / 2).max(0), area.top + ((area.height() - height) / 2).max(0))
}

/// Lays the page out at `scale` (DPI / 96).
pub(super) fn layout(scale: f64, measure: Measure<'_>, texts: &Texts) -> Layout {
    let s = |logical: i32| (f64::from(logical) * scale).round() as i32;
    let width_of = |face: Face, text: &str| measure(face, text, None).0;
    let wrapped = |text: &str, wrap: i32| measure(Face::Body, text, Some(wrap)).1;

    let line = measure(Face::Body, "Ag", None).1;
    let header_line = measure(Face::Header, "Ag", None).1;
    let row = s(ROW).max(line + s(ROW_PAD));
    let check = |text: &str| width_of(Face::Body, text) + s(CHECK_EXTRA);
    let button = |text: &str| (width_of(Face::Body, text) + s(BUTTON_PAD)).max(s(BUTTON_MIN));

    // Columns.
    let header_x = s(MARGIN);
    let label_x = header_x + s(INDENT);
    let label_w = widest(Part::LABELS.iter().map(|part| width_of(Face::Body, part.title())));
    let mode_label_w = width_of(Face::Header, Part::ModeLabel.title());
    let control_x = (label_x + label_w).max(header_x + mode_label_w) + s(COLUMN_GAP);

    // The control column is as wide as its widest row.
    let radios: Vec<i32> = (0..JiggleMode::MENU_ORDER.len()).map(|index| check(Part::Mode(index).title())).collect();
    let radios_w = radios.iter().sum::<i32>() + s(RADIO_GAP) * (radios.len() as i32 - 1);
    let day_w = widest(Weekday::ALL.iter().map(|day| check(day.short_name())));
    let days_w = day_w * 7 + s(DAY_GAP) * 6;
    let picker_w =
        widest(texts.times.iter().map(|time| width_of(Face::Body, time) + s(PICKER_EXTRA))).max(s(PICKER_MIN));
    let combo_w =
        widest(texts.choices.iter().map(|choice| width_of(Face::Body, choice) + s(COMBO_EXTRA))).max(s(COMBO_MIN));
    let control_w = widest([s(CONTROL_MIN), radios_w, days_w, picker_w, combo_w]);

    let width = control_x + control_w + s(MARGIN);
    let full_w = width - s(MARGIN) - label_x;
    let header_w = width - s(MARGIN) - header_x;

    let mut parts = Vec::with_capacity(Part::ORDER.len());
    let mut y = s(TOP);

    // Mode.
    parts.push((Part::ModeLabel, rect(header_x, y, control_x - s(COLUMN_GAP) - header_x, row)));
    let mut x = control_x;
    for (index, radio_w) in radios.iter().enumerate() {
        parts.push((Part::Mode(index), rect(x, y, *radio_w, row)));
        x += radio_w + s(RADIO_GAP);
    }
    y += row + s(TEXT_GAP);
    let mode_footer_h = widest(MODE_FOOTERS.iter().map(|footer| wrapped(footer, control_w)));
    parts.push((Part::ModeFooter, rect(control_x, y, control_w, mode_footer_h)));
    y += mode_footer_h + s(SECTION_GAP);

    // Schedule.
    parts.push((Part::ScheduleHeader, rect(header_x, y, header_w, header_line)));
    y += header_line + s(HEADER_GAP);
    parts.push((Part::DaysLabel, rect(label_x, y, label_w, row)));
    for index in 0..7 {
        parts.push((Part::Day(index), rect(control_x + index as i32 * (day_w + s(DAY_GAP)), y, day_w, row)));
    }
    y += row + s(ROW_GAP);
    parts.push((Part::FromLabel, rect(label_x, y, label_w, row)));
    parts.push((Part::From, rect(control_x, y, picker_w, row)));
    y += row + s(ROW_GAP);
    parts.push((Part::ToLabel, rect(label_x, y, label_w, row)));
    parts.push((Part::To, rect(control_x, y, picker_w, row)));
    y += row + s(TEXT_GAP);
    let hint_h = widest(texts.hints.iter().map(|hint| wrapped(hint, control_w)));
    parts.push((Part::Hint, rect(control_x, y, control_w, hint_h)));
    y += hint_h + s(SECTION_GAP);

    // Movement.
    parts.push((Part::MovementHeader, rect(header_x, y, header_w, header_line)));
    y += header_line + s(HEADER_GAP);
    parts.push((Part::DistanceLabel, rect(label_x, y, label_w, row)));
    parts.push((Part::Distance, rect(control_x, y, combo_w, row)));
    y += row + s(ROW_GAP);
    parts.push((Part::IntervalLabel, rect(label_x, y, label_w, row)));
    parts.push((Part::Interval, rect(control_x, y, combo_w, row)));
    y += row + s(ROW_GAP);
    // As on macOS: "Try the movement now" names the button, and the note
    // goes under that label.
    parts.push((Part::PreviewLabel, rect(label_x, y, label_w, row)));
    parts.push((Part::Preview, rect(control_x, y, button(Part::Preview.title()), row)));
    y += row + s(TEXT_GAP);
    let note_h = wrapped(PREVIEW_NOTE, full_w);
    parts.push((Part::PreviewNote, rect(label_x, y, full_w, note_h)));
    y += note_h + s(ROW_GAP);
    let footer_h = wrapped(MOVEMENT_FOOTER, full_w);
    parts.push((Part::MovementFooter, rect(label_x, y, full_w, footer_h)));
    y += footer_h + s(SECTION_GAP);

    // General.
    parts.push((Part::GeneralHeader, rect(header_x, y, header_w, header_line)));
    y += header_line + s(HEADER_GAP);
    parts.push((Part::Launch, rect(label_x, y, check(Part::Launch.title()).min(full_w), row)));
    y += row + s(CLOSE_GAP);
    let close_w = button(Part::Close.title());
    parts.push((Part::Close, rect(width - s(MARGIN) - close_w, y, close_w, row)));
    y += row + s(BOTTOM);

    Layout { width, height: y, parts }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A font whose glyphs are all `6 * scale` pixels wide (7 for headers)
    /// and whose lines are `16 * scale` tall, wrapping by width alone.
    fn fake_font(scale: f64) -> impl Fn(Face, &str, Option<i32>) -> (i32, i32) {
        move |face: Face, text: &str, wrap: Option<i32>| {
            let glyph = if face == Face::Header { 7.0 } else { 6.0 };
            let width = (text.chars().count() as f64 * glyph * scale).round() as i32;
            let line = (16.0 * scale).round() as i32;
            match wrap {
                None => (width, line),
                Some(wrap) => {
                    let lines = ((width + wrap - 1) / wrap).max(1);
                    (width.min(wrap), lines * line)
                }
            }
        }
    }

    fn texts() -> Texts {
        Texts {
            choices: ["Tiny · up to 2 pt", "Gentle · up to 5 pt", "Playful · up to 12 pt", "30 seconds", "4 minutes"]
                .map(str::to_owned)
                .to_vec(),
            times: vec!["10:58".to_owned(), "22:58".to_owned()],
            hints: crate::ui::settings_window::page::hint_samples(&|time| time.to_string()),
        }
    }

    fn at(scale: f64) -> Layout {
        layout(scale, &fake_font(scale), &texts())
    }

    #[test]
    fn every_part_is_laid_out_once_in_tab_order() {
        let layout = at(1.0);
        let parts: Vec<Part> = layout.parts.iter().map(|&(part, _)| part).collect();
        assert_eq!(parts, Part::ORDER);
    }

    #[test]
    fn the_page_at_100_percent() {
        let layout = at(1.0);
        // The label column is as wide as "Try the movement now" (20 glyphs,
        // 120 px), so controls start at 28 + 120 + 16.
        let preview_label = layout.rect(Part::PreviewLabel);
        assert_eq!((preview_label.left, preview_label.right, preview_label.height()), (28, 148, 24));
        assert_eq!(layout.rect(Part::ModeLabel), RectPx::new(16, 12, 148, 36));
        // The widest row, the days (7 x (3 glyphs + 22) + 6 x 2 = 292), is
        // under the minimum, so the control column is 320 wide.
        assert_eq!(layout.width, 164 + 320 + 16);
        assert_eq!(layout.rect(Part::Mode(0)), RectPx::new(164, 12, 164 + 66 + 22, 36));
        assert_eq!(layout.rect(Part::Mode(1)).left, 164 + 88 + 12);
        // The longest mode footer (59 glyphs, 354 px) wraps to two lines.
        assert_eq!(layout.rect(Part::ModeFooter), RectPx::new(164, 40, 484, 72));
        assert_eq!(layout.rect(Part::ScheduleHeader), RectPx::new(16, 88, 484, 104));
        assert_eq!(layout.rect(Part::Day(0)), RectPx::new(164, 110, 204, 134));
        assert_eq!(layout.rect(Part::Day(6)), RectPx::new(164 + 6 * 42, 110, 164 + 6 * 42 + 40, 134));
        assert_eq!(layout.rect(Part::From), RectPx::new(164, 140, 164 + 112, 164));
        assert_eq!(layout.rect(Part::To), RectPx::new(164, 170, 164 + 112, 194));
        // The "all day" hint (57 glyphs) needs two lines, so two are reserved.
        assert_eq!(layout.rect(Part::Hint), RectPx::new(164, 198, 484, 230));
        assert_eq!(layout.rect(Part::Distance), RectPx::new(164, 268, 164 + 208, 292));
        assert_eq!(layout.rect(Part::Interval), RectPx::new(164, 298, 164 + 208, 322));
        // "Try the movement now" names the button on its right, as on macOS...
        assert_eq!(preview_label.top, 328);
        assert_eq!(layout.rect(Part::Preview), RectPx::new(164, 328, 164 + 88, 352));
        // ...and the note (47 glyphs, one line) goes under the label.
        assert_eq!(layout.rect(Part::PreviewNote), RectPx::new(28, 356, 484, 372));
        let footer = layout.rect(Part::MovementFooter);
        assert_eq!((footer.left, footer.top, footer.right), (28, 378, 484));
        let close = layout.rect(Part::Close);
        assert_eq!((close.right, close.width(), close.height()), (layout.width - 16, 88, 24));
        assert_eq!(layout.height, close.bottom + 14);
    }

    #[test]
    fn the_page_at_150_percent_is_the_100_percent_page_scaled() {
        // Every logical constant is even and the fake font scales linearly,
        // so 150 % is exactly 1.5 times 100 %.
        let (small, large) = (at(1.0), at(1.5));
        let scaled = |value: i32| value * 3 / 2;
        assert_eq!(large.width, scaled(small.width));
        assert_eq!(large.height, scaled(small.height));
        for (&(part, a), &(_, b)) in small.parts.iter().zip(&large.parts) {
            let expected = RectPx::new(scaled(a.left), scaled(a.top), scaled(a.right), scaled(a.bottom));
            assert_eq!(b, expected, "{part:?}");
        }
    }

    #[test]
    fn parts_stay_inside_the_window_and_rows_do_not_overlap() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
            let layout = at(scale);
            let window = RectPx::new(0, 0, layout.width, layout.height);
            for &(part, rect) in &layout.parts {
                assert!(!rect.is_empty(), "{part:?} at {scale}");
                assert!(
                    rect.left >= window.left
                        && rect.top >= window.top
                        && rect.right <= window.right
                        && rect.bottom <= window.bottom,
                    "{part:?} at {scale}: {rect:?} outside {window:?}"
                );
            }
            let rows = [
                Part::Mode(0),
                Part::ModeFooter,
                Part::ScheduleHeader,
                Part::Day(0),
                Part::From,
                Part::To,
                Part::Hint,
                Part::MovementHeader,
                Part::Distance,
                Part::Interval,
                Part::Preview,
                Part::PreviewNote,
                Part::MovementFooter,
                Part::GeneralHeader,
                Part::Launch,
                Part::Close,
            ];
            for pair in rows.windows(2) {
                let (above, below) = (layout.rect(pair[0]), layout.rect(pair[1]));
                assert!(above.bottom <= below.top, "{:?} overlaps {:?} at {scale}", pair[0], pair[1]);
            }
            for row in [[Part::Mode(0), Part::Mode(1)], [Part::Mode(1), Part::Mode(2)]]
                .into_iter()
                .chain((0..6).map(|index| [Part::Day(index), Part::Day(index + 1)]))
            {
                let (left, right) = (layout.rect(row[0]), layout.rect(row[1]));
                assert!(left.right <= right.left && left.top == right.top, "{row:?} at {scale}");
            }
            let control_x = layout.rect(Part::Mode(0)).left;
            for label in Part::LABELS.into_iter().chain([Part::ModeLabel]) {
                assert!(layout.rect(label).right < control_x, "{label:?} reaches the controls at {scale}");
            }
            for part in [Part::ModeFooter, Part::Day(0), Part::From, Part::Hint, Part::Distance, Part::Preview] {
                assert_eq!(layout.rect(part).left, control_x, "{part:?} at {scale}");
            }
            // The Preview button shares its label's row; the note is under both.
            let (label, button, note) =
                (layout.rect(Part::PreviewLabel), layout.rect(Part::Preview), layout.rect(Part::PreviewNote));
            assert_eq!((label.top, label.bottom), (button.top, button.bottom), "Preview row at {scale}");
            assert!(note.top >= label.bottom && note.left == label.left, "Preview note at {scale}");
        }
    }

    #[test]
    fn windows_are_centered_in_the_work_area() {
        let work = RectPx::new(0, 0, 1920, 1040);
        assert_eq!(centered(work, 520, 640), (700, 200));
        // A monitor left of and above the primary one.
        let left = RectPx::new(-2560, -300, 0, 1100);
        assert_eq!(centered(left, 780, 960), (-2560 + 890, -300 + 220));
        // Too big for the area: the caption stays on screen.
        assert_eq!(centered(RectPx::new(100, 50, 600, 450), 800, 700), (100, 50));
    }

    #[test]
    fn a_wider_font_widens_the_window_instead_of_clipping() {
        let wide = |face: Face, text: &str, wrap: Option<i32>| {
            let (width, height) = fake_font(1.0)(face, text, wrap);
            match wrap {
                None => (width * 2, height),
                Some(wrap) => fake_font(1.0)(face, &format!("{text}{text}"), Some(wrap)),
            }
        };
        let (normal, large) = (at(1.0), layout(1.0, &wide, &texts()));
        assert!(large.width > normal.width);
        assert!(large.height > normal.height);
        for index in 0..7 {
            assert!(large.rect(Part::Day(index)).width() >= 2 * 3 * 6, "day {index}");
        }
    }
}
