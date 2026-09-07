//! Vertical field bands on the input word: 3, 1, 3, 2, 4 units crest to dim.
//!
//! The spec drawing is 13 units tall. `t` is 0 at the top of the word and 1
//! at the bottom. Palette color 0 is crest, then hover, lit, mid, dim.

use crate::engine::character::{CharId, EffectCharacter};
use crate::engine::terminal::Terminal;
use crate::utils::palette::Palette;

/// Crest, hover, lit, mid, dim — top to bottom.
pub const FIELD_BAND_UNITS: &[u32] = &[3, 1, 3, 2, 4];

pub fn field_band_rows() -> u32 {
    FIELD_BAND_UNITS.iter().sum()
}

/// Which band `t` (0 at the top, 1 at the bottom) falls in.
pub fn field_band_index(t: f64) -> usize {
    field_band_index_in(t, FIELD_BAND_UNITS)
}

pub fn field_band_index_in(t: f64, units: &[u32]) -> usize {
    let total: u32 = units.iter().copied().sum();
    if total == 0 || units.is_empty() {
        return 0;
    }
    let u = t.clamp(0.0, 1.0) * total as f64;
    let mut acc = 0.0;
    for (i, &n) in units.iter().enumerate() {
        acc += n as f64;
        if u < acc {
            return i;
        }
    }
    units.len() - 1
}

/// Color each input character from the palette by its visual row in the word.
///
/// `input_coord.row` is 1-based and grows up, so the largest row is the top.
/// Half-block art (`▄▀█`) encodes two spec pixels per terminal cell, the way
/// the 19-row wordmark is stored in the screensaver logo. `t` is measured in
/// those pixels so a full `█` is two units and a half block is one. Empty
/// halves (the top of a leading `▄`, the bottom of a trailing `▀`) do not
/// count, so the M peak cannot steal the crest from the other letters.
pub fn apply_field_bands(terminal: &mut Terminal, palette: &Palette) {
    let mut max_row = i64::MIN;
    let mut min_half = f64::INFINITY;
    let mut max_half = f64::NEG_INFINITY;
    for &id in &terminal.input_characters {
        let ch = &terminal.arena[id.0 as usize];
        if skip_char(ch) {
            continue;
        }
        max_row = max_row.max(ch.input_coord.row);
    }
    if max_row == i64::MIN {
        return;
    }
    for &id in &terminal.input_characters {
        let ch = &terminal.arena[id.0 as usize];
        if skip_char(ch) {
            continue;
        }
        let (lo, hi) = visual_half_range(max_row, ch);
        min_half = min_half.min(lo);
        max_half = max_half.max(hi);
    }
    let span = max_half - min_half;
    if span <= 0.0 {
        return;
    }
    let ids: Vec<CharId> = terminal.input_characters.clone();
    for id in ids {
        let ch = &mut terminal.arena[id.0 as usize];
        if skip_char(ch) {
            continue;
        }
        let (lo, hi) = visual_half_range(max_row, ch);
        let t = ((lo + hi) * 0.5 - min_half) / span;
        ch.animation.input_fg_color = Some(palette.color(field_band_index(t)));
        ch.uses_input_preexisting_colors = true;
    }
}

/// Occupied half-pixel range `[lo, hi)` from the top of the cell grid.
fn visual_half_range(max_row: i64, ch: &EffectCharacter) -> (f64, f64) {
    let display_line = (max_row - ch.input_coord.row) as f64;
    let (lo, hi) = block_half_span(&ch.input_symbol);
    (display_line * 2.0 + lo, display_line * 2.0 + hi)
}

/// How much of a terminal cell a glyph inks, in half-pixels from the cell top.
fn block_half_span(symbol: &str) -> (f64, f64) {
    match symbol.chars().next().unwrap_or('\0') {
        '▀' => (0.0, 1.0),
        '▄' => (1.0, 2.0),
        _ => (0.0, 2.0),
    }
}

fn skip_char(ch: &EffectCharacter) -> bool {
    ch.is_fill_character || ch.input_symbol.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_units_are_three_one_three_two_four() {
        assert_eq!(FIELD_BAND_UNITS, &[3, 1, 3, 2, 4]);
        assert_eq!(field_band_rows(), 13);
    }

    #[test]
    fn index_follows_crest_hover_lit_mid_dim() {
        assert_eq!(field_band_index(0.0), 0);
        assert_eq!(field_band_index(3.0 / 13.0 - 1e-9), 0);
        assert_eq!(field_band_index(3.0 / 13.0), 1);
        assert_eq!(field_band_index(4.0 / 13.0), 2);
        assert_eq!(field_band_index(7.0 / 13.0), 3);
        assert_eq!(field_band_index(9.0 / 13.0), 4);
        assert_eq!(field_band_index(1.0), 4);
    }

    #[test]
    fn apply_colors_thirteen_rows_crest_to_dim() {
        use crate::engine::terminal::{Terminal, TerminalConfig};
        use crate::utils::palette::Palette;

        let input = ["X"; 13].join("\n");
        let mut terminal = Terminal::new(
            &input,
            TerminalConfig {
                canvas_width: 1,
                canvas_height: 13,
                ignore_terminal_dimensions: true,
                ..Default::default()
            },
        )
        .unwrap();
        let palette = Palette::from_hex_list("aa0000,00aa00,0000aa,aaaa00,00aaaa").unwrap();
        apply_field_bands(&mut terminal, &palette);

        let mut by_row: Vec<(i64, crate::utils::graphics::Color)> = terminal
            .input_characters
            .iter()
            .map(|&id| {
                let ch = &terminal.arena[id.0 as usize];
                assert!(ch.uses_input_preexisting_colors);
                (ch.input_coord.row, ch.animation.input_fg_color.unwrap())
            })
            .collect();
        by_row.sort_by_key(|(row, _)| std::cmp::Reverse(*row));

        let expected = [0, 0, 0, 1, 2, 2, 2, 3, 3, 4, 4, 4, 4];
        assert_eq!(by_row.len(), 13);
        for (i, (row, color)) in by_row.iter().enumerate() {
            assert_eq!(*row, 13 - i as i64);
            assert_eq!(*color, palette.color(expected[i]));
        }
    }

    #[test]
    fn apply_colors_nineteen_rows_keep_unit_proportions() {
        use crate::engine::terminal::{Terminal, TerminalConfig};
        use crate::utils::palette::Palette;

        let input = ["X"; 19].join("\n");
        let mut terminal = Terminal::new(
            &input,
            TerminalConfig {
                canvas_width: 1,
                canvas_height: 19,
                ignore_terminal_dimensions: true,
                ..Default::default()
            },
        )
        .unwrap();
        let palette = Palette::from_hex_list("111111,222222,333333,444444,555555").unwrap();
        apply_field_bands(&mut terminal, &palette);

        let mut by_row: Vec<(i64, usize)> = terminal
            .input_characters
            .iter()
            .map(|&id| {
                let ch = &terminal.arena[id.0 as usize];
                let color = ch.animation.input_fg_color.unwrap();
                let idx = (0..5)
                    .find(|&i| palette.color(i) == color)
                    .expect("palette color");
                (ch.input_coord.row, idx)
            })
            .collect();
        by_row.sort_by_key(|(row, _)| std::cmp::Reverse(*row));

        for (i, (row, idx)) in by_row.iter().enumerate() {
            let t = (19.0 - *row as f64 + 0.5) / 19.0;
            assert_eq!(*idx, field_band_index(t), "row {i} t={t}");
        }
    }

    fn band_of(ch: &crate::engine::character::EffectCharacter, palette: &Palette) -> usize {
        let color = ch.animation.input_fg_color.unwrap();
        (0..5)
            .find(|&i| palette.color(i) == color)
            .expect("palette color")
    }

    #[test]
    fn half_blocks_follow_nineteen_pixel_rows() {
        use crate::engine::terminal::{Terminal, TerminalConfig};
        use crate::utils::palette::Palette;

        // Screensaver encoding of the 19-row wordmark: a lower-half peak on
        // line 0 (the M), then █ = two pixel rows, ▄▀ = one.
        let mut lines = vec!["  ▄  ".to_string(), "▄███▄".to_string()];
        for _ in 0..6 {
            lines.push("█████".to_string());
        }
        lines.push("▀███▀".to_string());
        lines.push("  █  ".to_string());
        let input = lines.join("\n");
        let mut terminal = Terminal::new(
            &input,
            TerminalConfig {
                canvas_width: 5,
                canvas_height: 10,
                ignore_terminal_dimensions: true,
                ..Default::default()
            },
        )
        .unwrap();
        let palette = Palette::from_hex_list("111111,222222,333333,444444,555555").unwrap();
        apply_field_bands(&mut terminal, &palette);

        let mut by_col_row: Vec<(i64, i64, usize, String)> = terminal
            .input_characters
            .iter()
            .map(|&id| {
                let ch = &terminal.arena[id.0 as usize];
                (
                    ch.input_coord.column,
                    ch.input_coord.row,
                    band_of(ch, &palette),
                    ch.input_symbol.clone(),
                )
            })
            .collect();
        by_col_row.sort_by_key(|&(c, r, _, _)| (c, std::cmp::Reverse(r)));

        // The M peak (top ▄) is crest.
        let peak_band = by_col_row
            .iter()
            .filter(|(_, _, _, s)| s.as_str() == "▄")
            .max_by_key(|(_, r, _, _)| *r)
            .unwrap();
        assert_eq!(peak_band.2, 0, "M peak must be crest, got {peak_band:?}");

        // A letter with no peak: the top-row █ (second input line) is still
        // crest. Character-row mapping would already have moved it to hover.
        let body_top = by_col_row
            .iter()
            .filter(|(_, _, _, s)| s.as_str() == "█")
            .max_by_key(|(_, r, _, _)| *r)
            .unwrap();
        assert_eq!(
            body_top.2, 0,
            "first full body row must stay crest, got {body_top:?}"
        );
    }
}
