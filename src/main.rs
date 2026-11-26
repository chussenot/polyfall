use macroquad::prelude::*;

mod fx;
use fx::{alpha, ease_out, Fx};

const W: i32 = 12;
const H: i32 = 22;
const CELL: f32 = 30.0;
const PANEL: f32 = 200.0;
const VIEW_W: f32 = W as f32 * CELL + PANEL;
const VIEW_H: f32 = H as f32 * CELL;
const TEXT: Color = Color::new(0.90, 0.91, 0.94, 1.0);
const DIM: Color = Color::new(0.50, 0.53, 0.62, 1.0);
const GLOW: f32 = 1.3; // seconds the level number stays highlighted
const GLOW_FADE: f32 = 0.7; // the last part of GLOW, spent fading; before that the highlight is at full strength
const BG: Color = Color::new(0.07, 0.08, 0.11, 1.0);
const WELL: Color = Color::new(0.12, 0.13, 0.18, 1.0);
// one colour per cell count (3, 4, 5): blue / orange / pale sand, rising in lightness so they also read without hue
const COLORS: [Color; 3] = [
    Color::new(0.36, 0.52, 0.85, 1.0),
    Color::new(0.88, 0.58, 0.36, 1.0),
    Color::new(0.92, 0.88, 0.70, 1.0),
];
const GUIDE: Color = Color::new(1.0, 1.0, 1.0, 0.025); // tint on every other column
const LANDING_ALPHA: f32 = 0.5;

type Grid = [[u8; W as usize]; H as usize];

/// Something that happened in the game, for the effects in `fx` to react to.
/// Events only describe; no game rule reads them.
#[derive(Debug, Clone, PartialEq)]
enum Event {
    /// The falling piece moved by this many cells: a key, a repeat, or one row of falling.
    Moved { dx: i32, dy: i32 },
    /// The falling piece made a quarter turn, and was kicked by this many cells to fit.
    Rotated { dx: i32, dy: i32 },
    /// The falling piece was dropped this many rows at once. `Locked` always follows.
    HardDropped { rows: i32 },
    /// The falling piece became part of the stack.
    Locked {
        cells: Vec<(i32, i32)>, // its cells, as (column, row) in the well
        color: u8,
        cleared: Vec<usize>, // the full rows this removed, ascending, as numbered before they went
        before: Box<Grid>,   // the well with the piece in it and those rows still there
        points: u32,
    },
    /// The level rose to `level`, counted from 0 like `Game::level`.
    LevelUp { level: u32 },
    /// The next piece had no room.
    GameOver,
}

struct Piece {
    cells: Vec<(i32, i32)>, // offsets from (x, y); always contains (0, 0), the rotation pivot
    x: i32,
    y: i32,
    color: u8, // 1-based index into COLORS; 0 means empty in the grid
}

/// Grows a random connected shape of 3 to 5 cells.
fn random_piece() -> Piece {
    let n = rand::gen_range(3, 6) as usize;
    let mut cells = vec![(0, 0)];
    while cells.len() < n {
        let (x, y) = cells[rand::gen_range(0, cells.len() as i32) as usize];
        let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][rand::gen_range(0, 4) as usize];
        if !cells.contains(&(x + dx, y + dy)) {
            cells.push((x + dx, y + dy));
        }
    }
    let top = cells.iter().map(|c| c.1).min().unwrap();
    Piece {
        cells,
        x: W / 2,
        y: -top,
        color: n as u8 - 2,
    }
}

struct Game {
    grid: Grid,
    piece: Piece,
    next: Piece,
    after: Piece, // the piece that follows `next`
    score: u32,
    lines: u32,
    fall: f32,
    held: f32, // seconds Left/Right has been held, for key repeat
    idle: f32, // seconds of active play with no key pressed or held
    over: bool,
    paused: bool,
    events: Vec<Event>, // what happened since `main` last took them; write-only for game logic
}

const REPEAT_DELAY: f32 = 0.17;
const REPEAT_RATE: f32 = 0.05;
const IDLE_PAUSE: f32 = 8.0; // seconds without a key before the game pauses itself

impl Game {
    fn new() -> Self {
        Game {
            grid: [[0; W as usize]; H as usize],
            piece: random_piece(),
            next: random_piece(),
            after: random_piece(),
            score: 0,
            lines: 0,
            fall: 0.0,
            held: 0.0,
            idle: 0.0,
            over: false,
            paused: false,
            events: Vec::new(),
        }
    }

    fn fits(&self, cells: &[(i32, i32)], x: i32, y: i32) -> bool {
        cells.iter().all(|&(cx, cy)| {
            let (gx, gy) = (x + cx, y + cy);
            (0..W).contains(&gx) && (0..H).contains(&gy) && self.grid[gy as usize][gx as usize] == 0
        })
    }

    fn shift(&mut self, dx: i32, dy: i32) -> bool {
        let ok = self.fits(&self.piece.cells, self.piece.x + dx, self.piece.y + dy);
        if ok {
            self.piece.x += dx;
            self.piece.y += dy;
            self.events.push(Event::Moved { dx, dy });
        }
        ok
    }

    /// Row the falling piece would rest on if hard-dropped.
    fn landing_y(&self) -> i32 {
        let p = &self.piece;
        let mut y = p.y;
        while self.fits(&p.cells, p.x, y + 1) {
            y += 1;
        }
        y
    }

    fn rotate(&mut self) {
        let turned: Vec<_> = self.piece.cells.iter().map(|&(x, y)| (-y, x)).collect();
        let (px, py) = (self.piece.x, self.piece.y);
        // a kick was stopped by the side wall alone: some cell left 0..W and no cell hit a locked block
        let wall_only = |k: i32| {
            let cells = || turned.iter().map(|&(cx, cy)| (px + cx + k, py + cy));
            cells().any(|(gx, _)| !(0..W).contains(&gx))
                && cells().all(|(gx, gy)| {
                    !(0..W).contains(&gx)
                        || !(0..H).contains(&gy)
                        || self.grid[gy as usize][gx as usize] == 0
                })
        };
        // nudge sideways if the turned shape hits a wall or the stack, else down one row (spawn row)
        // shortcut: a single row down, so a shape poking 2+ rows above row 0 still refuses; add more rows if that matters
        for (dx, dy) in [(0i32, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, 1)] {
            // two cells only to clear a wall, never to hop over stack blocks
            if dx.abs() == 2 && !wall_only(dx / 2) {
                continue;
            }
            if self.fits(&turned, px + dx, py + dy) {
                self.piece.x += dx;
                self.piece.y += dy;
                self.piece.cells = turned;
                self.events.push(Event::Rotated { dx, dy });
                return;
            }
        }
    }

    /// Drops the piece straight to its landing row and locks it there.
    fn hard_drop(&mut self) {
        let rows = self.landing_y() - self.piece.y;
        self.piece.y += rows;
        self.events.push(Event::HardDropped { rows });
        self.lock();
    }

    fn lock(&mut self) {
        let cells: Vec<_> = self
            .piece
            .cells
            .iter()
            .map(|&(cx, cy)| (self.piece.x + cx, self.piece.y + cy))
            .collect();
        for &(x, y) in &cells {
            self.grid[y as usize][x as usize] = self.piece.color;
        }
        let before = Box::new(self.grid);
        let kept: Vec<_> = self
            .grid
            .iter()
            .copied()
            .filter(|row| row.contains(&0))
            .collect();
        let rows: Vec<usize> = (0..H as usize)
            .filter(|&y| !self.grid[y].contains(&0))
            .collect();
        let cleared = rows.len();
        let level = self.level();
        let points = (cleared * cleared) as u32 * 100;
        self.grid = [[0; W as usize]; H as usize];
        self.grid[cleared..].copy_from_slice(&kept);
        self.lines += cleared as u32;
        self.score += points;
        self.events.push(Event::Locked {
            cells,
            color: self.piece.color,
            cleared: rows,
            before,
            points,
        });
        if self.level() > level {
            self.events.push(Event::LevelUp {
                level: self.level(),
            });
        }
        self.fall = 0.0;
        self.piece = std::mem::replace(
            &mut self.next,
            std::mem::replace(&mut self.after, random_piece()),
        );
        self.over = !self.fits(&self.piece.cells, self.piece.x, self.piece.y);
        if self.over {
            self.events.push(Event::GameOver);
        }
    }

    fn level(&self) -> u32 {
        self.lines / 10
    }

    /// R must not wipe a game in progress.
    fn can_restart(&self) -> bool {
        self.over || self.paused
    }

    /// Repeat moves owed when a key's held time grows from `before` to `after`.
    fn repeats(before: f32, after: f32) -> i32 {
        let due = |t: f32| {
            if t < REPEAT_DELAY {
                0
            } else {
                1 + ((t - REPEAT_DELAY) / REPEAT_RATE) as i32
            }
        };
        due(after) - due(before)
    }

    /// Pauses after `IDLE_PAUSE` seconds of play without a key; the count restarts once paused or over.
    fn tick_idle(&mut self, dt: f32, key_active: bool) {
        self.idle = if key_active || self.over || self.paused {
            0.0
        } else {
            self.idle + dt
        };
        self.paused |= self.idle >= IDLE_PAUSE;
    }

    fn update(&mut self) {
        let pressed = is_key_pressed(KeyCode::Right) as i32 - is_key_pressed(KeyCode::Left) as i32;
        let down = is_key_down(KeyCode::Right) as i32 - is_key_down(KeyCode::Left) as i32;
        if pressed != 0 {
            self.held = 0.0;
            self.shift(pressed, 0);
        } else if down != 0 {
            let held = self.held + get_frame_time();
            for _ in 0..Self::repeats(self.held, held) {
                self.shift(down, 0);
            }
            self.held = held;
        }
        if is_key_pressed(KeyCode::Up) {
            self.rotate();
        }
        if is_key_pressed(KeyCode::Space) {
            self.hard_drop();
            return;
        }
        self.fall += get_frame_time();
        let interval = if is_key_down(KeyCode::Down) {
            0.04
        } else {
            (0.6 - 0.05 * self.level() as f32).max(0.08)
        };
        if self.fall >= interval {
            self.fall = 0.0;
            if !self.shift(0, 1) {
                self.lock();
            }
        }
    }

    fn draw(&self, fx: &Fx) {
        // fixed logical view: miniquad shrinks the logical screen on scaled displays, which cropped the layout
        let view = |down: f32| {
            set_camera(&Camera2D::from_display_rect(Rect::new(
                0.0,
                VIEW_H - down,
                VIEW_W,
                -VIEW_H,
            )))
        };
        clear_background(BG);
        view(fx.kick_px()); // an impact knocks the well down a few units; the side panel stays put
        self.draw_well(fx);
        view(0.0);
        self.draw_panel(fx);
        self.draw_overlay(fx);
    }

    /// The well, the locked blocks and the falling piece, with their effects in between.
    fn draw_well(&self, fx: &Fx) {
        draw_rectangle(0.0, 0.0, W as f32 * CELL, H as f32 * CELL, WELL);
        for x in (1..W).step_by(2) {
            draw_rectangle(x as f32 * CELL, 0.0, CELL, H as f32 * CELL, GUIDE);
        }
        fx.draw_under();
        for (y, row) in self.grid.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                if c != 0 {
                    let (r, flash) = fx.block(x, y);
                    draw_rectangle(r.x, r.y, r.w, r.h, COLORS[c as usize - 1]);
                    if flash > 0.0 {
                        draw_rectangle(r.x, r.y, r.w, r.h, alpha(WHITE, flash));
                    }
                }
            }
        }
        fx.draw_clear();
        let p = &self.piece;
        let color = COLORS[p.color as usize - 1];
        // the outline follows the real piece, not the drawn one: it answers a key in the same frame
        let land = self.landing_y();
        if land != p.y {
            let outline = alpha(color, LANDING_ALPHA);
            for &(cx, cy) in &p.cells {
                let (x, y) = ((p.x + cx) as f32 * CELL, (land + cy) as f32 * CELL);
                draw_rectangle_lines(x + 2.0, y + 2.0, CELL - 4.0, CELL - 4.0, 2.0, outline);
            }
        }
        // drawn after the outline, so the piece covers it where they overlap
        let (slide, spin, flash) = fx.piece_pose();
        let moving = (slide.length() / CELL + spin.abs()).min(1.0);
        if moving > 0.02 {
            // a faint copy further back along the motion
            draw_piece(p, slide * 1.8, spin * 1.6, alpha(color, 0.2 * moving));
        }
        draw_piece(p, slide, spin, color);
        if flash > 0.0 {
            draw_piece(p, slide, spin, alpha(WHITE, flash));
        }
        fx.draw_over();
    }

    /// Everything right of the well: score, next piece, key list.
    fn draw_panel(&self, fx: &Fx) {
        let px = W as f32 * CELL + 20.0;
        draw_text("SCORE", px, 34.0, 16.0, DIM);
        if let Some((points, color, rise)) = fx.gain() {
            let gain = format!("+{points}");
            let w = measure_text(&gain, None, 16, 1.0).width;
            draw_text(&gain, px + 160.0 - w, 34.0 - rise, 16.0, color);
        }
        let score = fx.score(self.score).to_string();
        draw_text(&score, px, 74.0, 48.0, TEXT);
        draw_text(&score, px, 74.0, 48.0, fx.score_tint());
        draw_text("LINES", px, 112.0, 16.0, DIM);
        draw_text(self.lines.to_string(), px, 140.0, 28.0, TEXT);
        draw_text("LEVEL", px + 80.0, 112.0, 16.0, DIM);
        // the old number leaves upward while the new one comes in from below, highlighted
        let level = (self.level() + 1).to_string();
        let since = fx.levelled();
        let arrived = ease_out(since / fx::LEVEL_SLIDE);
        let (lx, ly) = (px + 80.0, 140.0 + 12.0 * (1.0 - arrived));
        if arrived < 1.0 {
            let old = self.level().to_string();
            draw_text(
                &old,
                lx,
                140.0 - 12.0 * arrived,
                28.0,
                alpha(TEXT, 1.0 - arrived),
            );
        }
        draw_text(&level, lx, ly, 28.0, alpha(TEXT, arrived));
        let glow = ((GLOW - since) / GLOW_FADE).clamp(0.0, 1.0);
        draw_text(&level, lx, ly, 28.0, alpha(COLORS[1], glow * arrived));
        // one line under the number, widening as it fades
        let pulse = since / 0.6;
        if pulse < 1.0 {
            let (w, grow) = (
                measure_text(&level, None, 28, 1.0).width,
                14.0 * ease_out(pulse),
            );
            let line = alpha(COLORS[1], 0.8 * (1.0 - pulse).powi(2));
            draw_rectangle(lx - grow, 148.0, w + 2.0 * grow, 2.0, line);
        }

        // a fixed square slot per upcoming piece; the piece (at most 5 cells either way) is centred in it
        let preview = |p: &Piece, by: f32, side: f32, cell: f32| {
            draw_rectangle(px, by, side, side, WELL);
            draw_rectangle_lines(px, by, side, side, 2.0, DIM);
            let span = |f: fn(&(i32, i32)) -> i32| {
                let (lo, hi) = (
                    p.cells.iter().map(f).min().unwrap(),
                    p.cells.iter().map(f).max().unwrap(),
                );
                (side - (hi - lo + 1) as f32 * cell) / 2.0 - lo as f32 * cell
            };
            let (ox, oy) = (px + span(|c| c.0), by + span(|c| c.1));
            for &(cx, cy) in &p.cells {
                let (x, y) = (ox + cx as f32 * cell, oy + cy as f32 * cell);
                draw_rectangle(
                    x + 1.0,
                    y + 1.0,
                    cell - 2.0,
                    cell - 2.0,
                    COLORS[p.color as usize - 1],
                );
            }
        };
        draw_text("NEXT", px, 182.0, 16.0, DIM);
        preview(&self.next, 192.0, 160.0, CELL);
        draw_text("THEN", px, 392.0, 14.0, DIM);
        preview(&self.after, 402.0, 80.0, CELL / 2.0);

        let keys = [
            ("Left/Right", "move"),
            ("Up", "rotate"),
            ("Down", "soft drop"),
            ("Space", "hard drop"),
            ("P", "pause"),
        ];
        for (i, (key, action)) in keys.iter().enumerate() {
            let y = 522.0 + i as f32 * 24.0;
            draw_text(key, px, y, 18.0, TEXT);
            draw_text(action, px + 96.0, y, 18.0, DIM);
        }
    }

    /// Drawn last, over the well: the paused / game-over panel.
    fn draw_overlay(&self, fx: &Fx) {
        let well_w = W as f32 * CELL;
        let lines = if self.over {
            [
                "Game over".to_string(),
                format!("Score {}", self.score),
                "R  new game".to_string(),
            ]
        } else if self.paused {
            [
                "Paused".to_string(),
                "P  resume".to_string(),
                "R  new game".to_string(),
            ]
        } else {
            return;
        };
        // a pause is instant; the end of a game is held, dimmed from the top down, then named
        let (dim, shown): (&dyn Fn(usize) -> f32, f32) = if self.over {
            (&|row| fx.over_dim(row), fx.over_box())
        } else {
            (&|_| 1.0, 1.0)
        };
        for row in 0..H as usize {
            let veil = alpha(BG, 0.7 * dim(row));
            draw_rectangle(0.0, row as f32 * CELL, well_w, CELL, veil);
        }
        if shown == 0.0 {
            return;
        }
        let (w, h) = (260.0, 150.0);
        let (x, y) = (
            (well_w - w) / 2.0,
            (VIEW_H - h) / 2.0 + 10.0 * (1.0 - shown),
        );
        draw_rectangle(x, y, w, h, alpha(BG, shown));
        draw_rectangle_lines(x, y, w, h, 2.0, alpha(DIM, shown));
        for (line, (dy, size, color)) in
            lines
                .iter()
                .zip([(52.0, 40.0, TEXT), (92.0, 24.0, TEXT), (124.0, 20.0, DIM)])
        {
            let tw = measure_text(line, None, size as u16, 1.0).width;
            draw_text(line, (well_w - tw) / 2.0, y + dy, size, alpha(color, shown));
        }
    }
}

/// The falling piece, `slide` view units from its real cells and turned by `spin` around its pivot.
fn draw_piece(p: &Piece, slide: Vec2, spin: f32, color: Color) {
    let pivot = vec2(p.x as f32 + 0.5, p.y as f32 + 0.5) * CELL + slide;
    for &(cx, cy) in &p.cells {
        let at = pivot + Vec2::from_angle(spin).rotate(vec2(cx as f32, cy as f32) * CELL);
        draw_rectangle_ex(
            at.x,
            at.y,
            CELL - 2.0,
            CELL - 2.0,
            DrawRectangleParams {
                offset: vec2(0.5, 0.5),
                rotation: spin,
                color,
            },
        );
    }
}

fn conf() -> Conf {
    Conf {
        window_title: "Polyfall".into(),
        window_width: VIEW_W as i32,
        window_height: VIEW_H as i32,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    rand::srand(macroquad::miniquad::date::now() as u64);
    let mut game = Game::new();
    let mut fx = Fx::default();
    loop {
        if is_key_pressed(KeyCode::R) && game.can_restart() {
            game = Game::new();
            fx = Fx::default();
        }
        if is_key_pressed(KeyCode::P) && !game.over {
            game.paused = !game.paused;
        }
        game.tick_idle(get_frame_time(), !get_keys_down().is_empty());
        if !game.over && !game.paused {
            game.update();
        }
        // effects only listen: they take what happened and never hand anything back
        for event in game.events.drain(..) {
            fx.on(event);
        }
        if !game.paused {
            fx.tick(get_frame_time()); // a pause freezes effects with the game; the end does not
        }
        game.draw(&fx);
        next_frame().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_pieces_spawn_inside_the_well() {
        let g = Game::new();
        for _ in 0..1000 {
            let p = random_piece();
            assert!((3..=5).contains(&p.cells.len()));
            assert!(g.fits(&p.cells, p.x, p.y));
        }
    }

    #[test]
    fn landing_row_stops_on_the_stack_or_the_floor() {
        let mut g = Game::new();
        g.grid[15][6] = 1;
        g.piece = Piece {
            cells: vec![(0, 0), (1, 0), (0, 1)],
            x: 5,
            y: 2,
            color: 1,
        };
        assert_eq!(g.landing_y(), 14); // the (1, 0) cell rests on the block at row 15
        g.piece.x = 2;
        assert_eq!(g.landing_y(), H - 2);
        g.piece.y = H - 2;
        assert_eq!(g.landing_y(), g.piece.y);
    }

    #[test]
    fn colour_follows_cell_count() {
        for _ in 0..1000 {
            let p = random_piece();
            assert_eq!(p.color as usize, p.cells.len() - 2);
        }
    }

    #[test]
    fn full_row_clears_and_stack_drops() {
        let mut g = Game::new();
        let bottom = H as usize - 1;
        g.grid[bottom] = [1; W as usize];
        g.grid[bottom][0] = 0;
        g.grid[bottom - 1][5] = 2;
        g.piece = Piece {
            cells: vec![(0, 0)],
            x: 0,
            y: H - 1,
            color: 3,
        };
        g.lock();
        assert_eq!((g.lines, g.score), (1, 100));
        assert_eq!(g.grid[bottom][5], 2);
        assert_eq!(g.grid[bottom].iter().filter(|&&c| c != 0).count(), 1);
        assert!(!g.over);
    }

    #[test]
    fn blocked_spawn_area_ends_the_game() {
        let mut g = Game::new();
        for row in &mut g.grid[..5] {
            *row = [1; W as usize];
            row[0] = 0; // gap so the row is not cleared; spawns never reach column 0
        }
        g.piece = Piece {
            cells: vec![(0, 0)],
            x: 0,
            y: H - 1,
            color: 3,
        };
        g.lock();
        assert_eq!(g.lines, 0);
        assert!(g.over);
    }

    #[test]
    fn lock_reports_cleared_rows_and_level_up() {
        let mut g = Game::new();
        let bottom = H as usize - 1;
        for row in &mut g.grid[bottom - 1..] {
            *row = [1; W as usize];
            row[0] = 0;
        }
        g.lines = 9;
        g.piece = Piece {
            cells: vec![(0, 0), (0, 1)],
            x: 0,
            y: H - 2,
            color: 3,
        };
        g.lock();
        assert_eq!((g.lines, g.score), (11, 400));
        let mut before = [[0; W as usize]; H as usize];
        before[bottom - 1] = [1; W as usize];
        before[bottom] = [1; W as usize];
        (before[bottom - 1][0], before[bottom][0]) = (3, 3);
        assert_eq!(
            g.events,
            vec![
                Event::Locked {
                    cells: vec![(0, H - 2), (0, H - 1)],
                    color: 3,
                    cleared: vec![bottom - 1, bottom],
                    before: Box::new(before),
                    points: 400,
                },
                Event::LevelUp { level: 1 },
            ]
        );

        g.events.clear();
        g.piece = Piece {
            cells: vec![(0, 0)],
            x: 0,
            y: H - 1,
            color: 3,
        };
        g.lock();
        assert!(matches!(
            &g.events[..],
            [Event::Locked { cleared, points: 0, .. }] if cleared.is_empty()
        ));
    }

    #[test]
    fn score_is_rows_squared_times_100() {
        for n in 1..=5usize {
            let mut g = Game::new();
            for row in &mut g.grid[H as usize - n..] {
                *row = [1; W as usize];
                row[0] = 0;
            }
            g.piece = Piece {
                cells: (0..n as i32).map(|i| (0, i)).collect(),
                x: 0,
                y: H - n as i32,
                color: 3,
            };
            g.lock();
            assert_eq!((g.lines, g.score), (n as u32, (n * n) as u32 * 100));
            assert_eq!(g.grid, [[0; W as usize]; H as usize]);
        }
    }

    #[test]
    fn moves_and_turns_are_reported_only_when_they_happen() {
        let mut g = Game::new();
        g.piece = Piece {
            cells: vec![(0, -1), (0, 0), (0, 1), (0, 2)],
            x: 0,
            y: 10,
            color: 1,
        };
        assert!(!g.shift(-1, 0)); // the wall
        assert!(g.events.is_empty());
        assert!(g.shift(1, 0));
        assert!(g.shift(0, 1));
        g.shift(-1, 0);
        g.rotate(); // kicked two cells off the wall
        assert_eq!(
            g.events,
            vec![
                Event::Moved { dx: 1, dy: 0 },
                Event::Moved { dx: 0, dy: 1 },
                Event::Moved { dx: -1, dy: 0 },
                Event::Rotated { dx: 2, dy: 0 },
            ]
        );
    }

    #[test]
    fn hard_drop_lands_where_the_outline_was_and_reports_the_distance() {
        let mut g = Game::new();
        g.grid[15][6] = 1;
        g.piece = Piece {
            cells: vec![(0, 0), (1, 0), (0, 1)],
            x: 5,
            y: 2,
            color: 1,
        };
        let land = g.landing_y() as usize;
        g.hard_drop();
        assert_eq!(
            (g.grid[land][5], g.grid[land][6], g.grid[land + 1][5]),
            (1, 1, 1)
        );
        assert_eq!(
            g.events[0],
            Event::HardDropped {
                rows: land as i32 - 2
            }
        );
        assert!(
            matches!(&g.events[1], Event::Locked { cells, .. } if cells[0] == (5, land as i32))
        );
        assert_eq!((g.events.len(), g.fall), (2, 0.0));
    }

    #[test]
    fn the_end_of_the_game_is_reported_once_after_the_last_lock() {
        let mut g = Game::new();
        for row in &mut g.grid[..5] {
            *row = [1; W as usize];
            row[0] = 0;
        }
        g.piece = Piece {
            cells: vec![(0, 0)],
            x: 0,
            y: H - 1,
            color: 3,
        };
        g.lock();
        assert!(matches!(
            &g.events[..],
            [Event::Locked { .. }, Event::GameOver]
        ));
    }

    #[test]
    fn lock_shifts_the_two_upcoming_pieces() {
        let mut g = Game::new();
        let (next, after) = (g.next.cells.clone(), g.after.cells.clone());
        g.piece = Piece {
            cells: vec![(0, 0)],
            x: 0,
            y: H - 1,
            color: 3,
        };
        g.lock();
        assert_eq!((g.piece.cells.clone(), g.next.cells.clone()), (next, after));
    }

    #[test]
    fn four_turns_restore_the_piece() {
        let mut g = Game::new();
        let cells = vec![(0, 0), (1, 0), (0, 1), (0, 2)];
        g.piece = Piece {
            cells: cells.clone(),
            x: 5,
            y: 10,
            color: 1,
        };
        g.rotate();
        assert_ne!(g.piece.cells, cells);
        for _ in 0..3 {
            g.rotate();
        }
        assert_eq!((&g.piece.cells, g.piece.x, g.piece.y), (&cells, 5, 10));
    }

    #[test]
    fn rotation_kicks_off_walls_but_not_over_the_stack() {
        // in a one-cell shaft between two one-cell columns: turning would need a hop over a column
        let mut g = Game::new();
        for row in &mut g.grid[9..=12] {
            row[4] = 1;
            row[6] = 1;
        }
        let cells = vec![(0, 0), (0, 1)];
        g.piece = Piece {
            cells: cells.clone(),
            x: 5,
            y: 10,
            color: 1,
        };
        g.rotate();
        assert_eq!((&g.piece.cells, g.piece.x, g.piece.y), (&cells, 5, 10));

        // a long bar against the left wall still turns, two cells away from it
        let mut g = Game::new();
        g.piece = Piece {
            cells: vec![(0, -1), (0, 0), (0, 1), (0, 2)],
            x: 0,
            y: 10,
            color: 1,
        };
        g.rotate();
        assert_eq!(g.piece.cells, vec![(1, 0), (0, 0), (-1, 0), (-2, 0)]);
        assert_eq!((g.piece.x, g.piece.y), (2, 10));
    }

    #[test]
    fn top_row_piece_can_still_turn() {
        let mut g = Game::new();
        g.piece = Piece {
            cells: vec![(0, 0), (-1, 0), (1, 0)],
            x: 5,
            y: 0,
            color: 1,
        };
        g.rotate();
        assert_eq!(g.piece.cells, vec![(0, 0), (0, -1), (0, 1)]);
        assert_eq!((g.piece.x, g.piece.y), (5, 1));
    }

    #[test]
    fn restart_only_when_over_or_paused() {
        let mut g = Game::new();
        assert!(!g.can_restart());
        g.paused = true;
        assert!(g.can_restart());
        g.paused = false;
        g.over = true;
        assert!(g.can_restart());
    }

    #[test]
    fn held_key_repeats_after_a_delay() {
        assert_eq!(Game::repeats(0.0, 0.16), 0);
        assert_eq!(Game::repeats(0.16, 0.18), 1);
        assert_eq!(Game::repeats(0.18, 0.20), 0);
        assert_eq!(Game::repeats(0.0, 0.28), 3); // one long frame owes every step it skipped
    }

    #[test]
    fn idle_play_pauses_itself() {
        let mut g = Game::new();
        g.tick_idle(5.0, false);
        assert_eq!((g.idle, g.paused), (5.0, false));
        g.tick_idle(0.1, true); // a key restarts the count
        g.tick_idle(5.0, false);
        assert_eq!((g.idle, g.paused), (5.0, false));
        g.tick_idle(3.0, false);
        assert!(g.paused);
        g.tick_idle(0.1, false); // paused: the count restarts, so resuming gets a full 8 s
        g.paused = false;
        g.tick_idle(0.1, false);
        assert_eq!((g.idle, g.paused), (0.1, false));

        let mut g = Game::new();
        g.over = true;
        g.tick_idle(60.0, false);
        assert!(!g.paused);
    }
}
