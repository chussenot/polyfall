//! The title screen: the name, the best score and the keys, over random pieces drifting down
//! the well. One key from here to play.
//!
//! It only draws. `main` reads the keys and owns the music; nothing here touches a game.

use crate::fx::{alpha, ease_out};
use crate::{random_piece, Piece, BG, CELL, COLORS, DIM, GUIDE, PANEL, TEXT, VIEW_H, W, WELL};
use macroquad::prelude::*;

const SPAWN: f32 = 0.9; // seconds between two drifting pieces
const DRIFT: (f32, f32) = (45.0, 80.0); // their speed, view units per second
const FADE_IN: f32 = 0.8; // seconds the name takes to appear

/// A piece drifting down behind the name, `y` view units from the top.
struct Drift {
    piece: Piece,
    y: f32,
    speed: f32,
}

#[derive(Default)]
pub struct Title {
    t: f32,
    since_spawn: f32,
    drifts: Vec<Drift>,
}

impl Title {
    pub fn tick(&mut self, dt: f32) {
        if self.t == 0.0 {
            // the screen opens with the well already in motion, not empty
            for _ in 0..6 {
                self.spawn(rand::gen_range(-4.0 * CELL, VIEW_H - 4.0 * CELL));
            }
        }
        self.t += dt;
        self.since_spawn += dt;
        if self.since_spawn >= SPAWN || self.drifts.is_empty() {
            self.since_spawn = 0.0;
            self.spawn(-5.0 * CELL);
        }
        for d in &mut self.drifts {
            d.y += d.speed * dt;
        }
        self.drifts.retain(|d| d.y < VIEW_H + CELL);
    }

    /// A new random piece in a random column, its top `y` view units down.
    fn spawn(&mut self, y: f32) {
        let mut piece = random_piece();
        let left = piece.cells.iter().map(|c| c.0).min().unwrap();
        let right = piece.cells.iter().map(|c| c.0).max().unwrap();
        piece.x = rand::gen_range(-left, W - right);
        self.drifts.push(Drift {
            piece,
            y,
            speed: rand::gen_range(DRIFT.0, DRIFT.1),
        });
    }

    pub fn draw(&self, best: u32, muted: bool) {
        set_camera(&Camera2D::from_display_rect(Rect::new(
            0.0,
            VIEW_H,
            W as f32 * CELL + PANEL,
            -VIEW_H,
        )));
        clear_background(BG);
        let well_w = W as f32 * CELL;
        draw_rectangle(0.0, 0.0, well_w, VIEW_H, WELL);
        for x in (0..W).step_by(2) {
            draw_rectangle(x as f32 * CELL, 0.0, CELL, VIEW_H, GUIDE);
        }
        for d in &self.drifts {
            let color = alpha(COLORS[d.piece.color as usize - 1], 0.35);
            for &(cx, cy) in &d.piece.cells {
                let (x, y) = ((d.piece.x + cx) as f32 * CELL, d.y + cy as f32 * CELL);
                draw_rectangle(x + 1.0, y + 1.0, CELL - 2.0, CELL - 2.0, color);
            }
        }

        // the name, one colour per letter, cycling through the three piece colours
        let shown = ease_out((self.t / FADE_IN).min(1.0));
        let name = "POLYFALL";
        let size = 64u16;
        let width = measure_text(name, None, size, 1.0).width;
        let (mut x, y) = ((well_w - width) / 2.0, VIEW_H * 0.38 + 12.0 * (1.0 - shown));
        draw_rectangle(0.0, y - 70.0, well_w, 100.0, alpha(BG, 0.75 * shown));
        for (i, letter) in name.chars().enumerate() {
            let s = letter.to_string();
            draw_text(&s, x, y, size as f32, alpha(COLORS[i % 3], shown));
            x += measure_text(&s, None, size, 1.0).width;
        }
        let tag = "random shapes, one well";
        let tw = measure_text(tag, None, 20, 1.0).width;
        draw_text(tag, (well_w - tw) / 2.0, y + 22.0, 20.0, alpha(DIM, shown));

        // the call to play pulses gently, so the eye finds it
        let call = "Enter  play";
        let pulse = 0.65 + 0.35 * (self.t * 2.4).sin().abs();
        let cw = measure_text(call, None, 28, 1.0).width;
        draw_text(
            call,
            (well_w - cw) / 2.0,
            VIEW_H * 0.62,
            28.0,
            alpha(TEXT, pulse * shown),
        );

        // the panel: the best score and the keys
        let px = well_w + 20.0;
        draw_text("BEST", px, 34.0, 16.0, DIM);
        draw_text(best.to_string(), px, 74.0, 48.0, TEXT);
        let keys = [
            ("Enter", "play"),
            ("M", if muted { "sound on" } else { "mute" }),
            ("Esc", "quit"),
        ];
        for (i, (key, action)) in keys.iter().enumerate() {
            let y = 522.0 + i as f32 * 24.0;
            draw_text(key, px, y, 18.0, TEXT);
            draw_text(action, px + 96.0, y, 18.0, DIM);
        }
    }
}
