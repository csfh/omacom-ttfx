use ttfx::engine::terminal::{PackedFrame, Terminal, TerminalConfig};

#[test]
fn pack_display_frame_puts_the_top_row_first() {
    let mut terminal = Terminal::new(
        "AB",
        TerminalConfig {
            canvas_width: 2,
            canvas_height: 1,
            ignore_terminal_dimensions: true,
            ..Default::default()
        },
    )
    .unwrap();
    let ids = terminal.input_characters.clone();
    for id in ids {
        terminal.set_character_visibility(id, true);
    }
    let packed = terminal.pack_display_frame();
    assert_eq!(packed.width, 2);
    assert_eq!(packed.height, 1);
    assert_eq!(packed.symbols, "AB");
    assert_eq!(packed.fg.len(), 2);
    assert_eq!(packed.bg.len(), 2);
    assert_eq!(packed.flags.len(), 2);
}

#[test]
fn pack_display_frame_empty_cells_are_spaces() {
    let mut terminal = Terminal::new(
        "A",
        TerminalConfig {
            canvas_width: 3,
            canvas_height: 1,
            ignore_terminal_dimensions: true,
            ..Default::default()
        },
    )
    .unwrap();
    let ids = terminal.input_characters.clone();
    for id in ids {
        terminal.set_character_visibility(id, true);
    }
    let packed = terminal.pack_display_frame();
    assert_eq!(packed.width, 3);
    assert_eq!(packed.symbols.chars().count(), 3);
    assert!(packed.symbols.contains('A'));
    assert!(packed.symbols.contains(' '));
    let _ = PackedFrame::BOLD;
}
