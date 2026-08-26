//! Browser bindings: run any effect against a packed cell frame.

use clap::{CommandFactory, Parser};
use wasm_bindgen::prelude::*;

use crate::cli::Cli;
use crate::engine::canvas::Anchor;
use crate::engine::ctx::{Clock, EngineCtx};
use crate::engine::effect::Effect;
use crate::engine::terminal::{PackedFrame, TerminalConfig};
use crate::utils::graphics::Color;
use crate::utils::rng::Rng;

#[wasm_bindgen]
pub struct Session {
    effect: Box<dyn Effect>,
    ctx: EngineCtx,
    frame: PackedFrame,
    done: bool,
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new(
        input: &str,
        effect: &str,
        columns: u32,
        rows: u32,
        seed: Option<f64>,
        frame_rate: u32,
    ) -> Result<Session, JsError> {
        if input.trim().is_empty() {
            return Err(JsError::new("NO INPUT."));
        }
        let columns = columns.max(1) as i64;
        let rows = rows.max(1) as i64;
        let frame_rate = frame_rate as i64;
        let rng = match seed {
            Some(s) if s.is_finite() => Rng::seeded(s as u64),
            _ => Rng::from_entropy(),
        };
        let config = TerminalConfig {
            frame_rate,
            canvas_width: 0,
            canvas_height: 0,
            anchor_canvas: Anchor::C,
            anchor_text: Anchor::C,
            reuse_canvas: true,
            no_eol: true,
            no_restore_cursor: true,
            terminal_background_color: Color::from_hex("000000").unwrap(),
            terminal_size: Some((columns, rows)),
            ..TerminalConfig::default()
        };
        let clock = Clock::virtual_with_frame_rate(if frame_rate > 0 { frame_rate } else { 60 });
        let mut ctx = EngineCtx::new(input, config, rng, clock).map_err(|e| JsError::new(&e.to_string()))?;
        let mut effect = build_effect(effect)?;
        effect.build(&mut ctx).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Session {
            effect,
            ctx,
            frame: PackedFrame {
                width: 0,
                height: 0,
                symbols: String::new(),
                fg: Vec::new(),
                bg: Vec::new(),
                flags: Vec::new(),
            },
            done: false,
        })
    }

    /// Advance one animation frame. Returns false when the effect is finished.
    pub fn step(&mut self) -> bool {
        if self.done {
            return false;
        }
        match self.effect.next_frame(&mut self.ctx) {
            Some(output) => {
                self.ctx.terminal.recycle_output_string(output);
                self.frame = self.ctx.terminal.pack_display_frame();
                true
            }
            None => {
                self.done = true;
                false
            }
        }
    }

    pub fn done(&self) -> bool {
        self.done
    }

    pub fn width(&self) -> u32 {
        self.frame.width as u32
    }

    pub fn height(&self) -> u32 {
        self.frame.height as u32
    }

    pub fn symbols(&self) -> String {
        self.frame.symbols.clone()
    }

    pub fn fg(&self) -> Vec<u32> {
        self.frame.fg.clone()
    }

    pub fn bg(&self) -> Vec<u32> {
        self.frame.bg.clone()
    }

    pub fn flags(&self) -> Vec<u8> {
        self.frame.flags.clone()
    }
}

fn build_effect(name: &str) -> Result<Box<dyn Effect>, JsError> {
    match Cli::try_parse_from(["ttfx", name]) {
        Ok(Cli { effect: Some(effect), .. }) => Ok(effect.build_effect()),
        Ok(_) => Err(JsError::new(&format!("unknown effect '{name}'"))),
        Err(e) => Err(JsError::new(&format!("unknown effect '{name}': {e}"))),
    }
}

/// JSON array of `{name, about}` for every registered effect.
#[wasm_bindgen]
pub fn effect_catalog() -> String {
    let mut out = String::from("[");
    for (i, cmd) in Cli::command().get_subcommands().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":");
        json_string(&mut out, cmd.get_name());
        out.push_str(",\"about\":");
        json_string(
            &mut out,
            cmd.get_about().map(|s| s.to_string()).unwrap_or_default().as_str(),
        );
        out.push('}');
    }
    out.push(']');
    out
}

fn json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str("\\u");
                let code = c as u32;
                let hex = format!("{code:04x}");
                out.push_str(&hex);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
