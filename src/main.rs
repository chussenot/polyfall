use macroquad::prelude::*;

const W: i32 = 12;
const H: i32 = 22;
const CELL: f32 = 30.0;
const PANEL: f32 = 200.0;
const VIEW_W: f32 = W as f32 * CELL + PANEL;
const VIEW_H: f32 = H as f32 * CELL;
const TEXT: Color = Color::new(0.90, 0.91, 0.94, 1.0);
const DIM: Color = Color::new(0.50, 0.53, 0.62, 1.0);
const FLASH: f32 = 0.25; // seconds the cleared-row band takes to fade
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
    Piece { cells, x: W / 2, y: -top, color: n as u8 - 2 }
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
    over: bool,
    paused: bool,
    cleared: Vec<usize>, // rows removed by the last lock, as they were before the collapse
    flash: f32,          // time left on the cleared-row band
    glow: f32,           // time left on the level highlight
}

const REPEAT_DELAY: f32 = 0.17;
const REPEAT_RATE: f32 = 0.05;

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
            over: false,
            paused: false,
            cleared: Vec::new(),
            flash: 0.0,
            glow: 0.0,
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
                    !(0..W).contains(&gx) || !(0..H).contains(&gy) || self.grid[gy as usize][gx as usize] == 0
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
                return;
            }
        }
    }

    fn lock(&mut self) {
        for &(cx, cy) in &self.piece.cells {
            self.grid[(self.piece.y + cy) as usize][(self.piece.x + cx) as usize] = self.piece.color;
        }
        let kept: Vec<_> = self.grid.iter().copied().filter(|row| row.contains(&0)).collect();
        self.cleared = (0..H as usize).filter(|&y| !self.grid[y].contains(&0)).collect();
        let cleared = self.cleared.len();
        let level = self.level();
        self.grid = [[0; W as usize]; H as usize];
        self.grid[cleared..].copy_from_slice(&kept);
        self.lines += cleared as u32;
        self.score += (cleared * cleared) as u32 * 100;
        if cleared > 0 {
            self.flash = FLASH;
        }
        if self.level() > level {
            self.glow = GLOW;
        }
        self.fall = 0.0;
        self.piece = std::mem::replace(&mut self.next, std::mem::replace(&mut self.after, random_piece()));
        self.over = !self.fits(&self.piece.cells, self.piece.x, self.piece.y);
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
        let due = |t: f32| if t < REPEAT_DELAY { 0 } else { 1 + ((t - REPEAT_DELAY) / REPEAT_RATE) as i32 };
        due(after) - due(before)
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
            while self.shift(0, 1) {}
            self.lock();
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

    fn draw(&mut self) {
        let dt = get_frame_time();
        self.flash = (self.flash - dt).max(0.0);
        self.glow = (self.glow - dt).max(0.0);
        // fixed logical view: miniquad shrinks the logical screen on scaled displays, which cropped the layout
        set_camera(&Camera2D::from_display_rect(Rect::new(0.0, VIEW_H, VIEW_W, -VIEW_H)));
        clear_background(BG);
        self.draw_well();
        self.draw_panel();
        self.draw_overlay();
    }

    /// The well, the locked blocks and the falling piece.
    fn draw_well(&self) {
        draw_rectangle(0.0, 0.0, W as f32 * CELL, H as f32 * CELL, WELL);
        for x in (1..W).step_by(2) {
            draw_rectangle(x as f32 * CELL, 0.0, CELL, H as f32 * CELL, GUIDE);
        }
        for (y, row) in self.grid.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                if c != 0 {
                    draw_cell(x as f32 * CELL, y as f32 * CELL, c);
                }
            }
        }
        let p = &self.piece;
        let land = self.landing_y();
        if land != p.y {
            let outline = Color { a: LANDING_ALPHA, ..COLORS[p.color as usize - 1] };
            for &(cx, cy) in &p.cells {
                let (x, y) = ((p.x + cx) as f32 * CELL, (land + cy) as f32 * CELL);
                draw_rectangle_lines(x + 2.0, y + 2.0, CELL - 4.0, CELL - 4.0, 2.0, outline);
            }
        }
        // drawn after the outline, so the real piece covers it where they overlap
        for &(cx, cy) in &p.cells {
            draw_cell((p.x + cx) as f32 * CELL, (p.y + cy) as f32 * CELL, p.color);
        }
    }

    /// Everything right of the well: score, next piece, key list.
    fn draw_panel(&self) {
        let px = W as f32 * CELL + 20.0;
        draw_text("SCORE", px, 34.0, 16.0, DIM);
        draw_text(&self.score.to_string(), px, 74.0, 48.0, TEXT);
        draw_text("LINES", px, 112.0, 16.0, DIM);
        draw_text(&self.lines.to_string(), px, 140.0, 28.0, TEXT);
        let level = (self.level() + 1).to_string();
        draw_text("LEVEL", px + 80.0, 112.0, 16.0, DIM);
        draw_text(&level, px + 80.0, 140.0, 28.0, TEXT);
        draw_text(&level, px + 80.0, 140.0, 28.0, Color { a: (self.glow / GLOW_FADE).min(1.0), ..COLORS[1] });

        // a fixed square slot per upcoming piece; the piece (at most 5 cells either way) is centred in it
        let preview = |p: &Piece, by: f32, side: f32, cell: f32| {
            draw_rectangle(px, by, side, side, WELL);
            draw_rectangle_lines(px, by, side, side, 2.0, DIM);
            let span = |f: fn(&(i32, i32)) -> i32| {
                let (lo, hi) = (p.cells.iter().map(f).min().unwrap(), p.cells.iter().map(f).max().unwrap());
                (side - (hi - lo + 1) as f32 * cell) / 2.0 - lo as f32 * cell
            };
            let (ox, oy) = (px + span(|c| c.0), by + span(|c| c.1));
            for &(cx, cy) in &p.cells {
                let (x, y) = (ox + cx as f32 * cell, oy + cy as f32 * cell);
                draw_rectangle(x + 1.0, y + 1.0, cell - 2.0, cell - 2.0, COLORS[p.color as usize - 1]);
            }
        };
        draw_text("NEXT", px, 182.0, 16.0, DIM);
        preview(&self.next, 192.0, 160.0, CELL);
        draw_text("THEN", px, 392.0, 14.0, DIM);
        preview(&self.after, 402.0, 80.0, CELL / 2.0);

        let keys = [("Left/Right", "move"), ("Up", "rotate"), ("Down", "soft drop"), ("Space", "hard drop"), ("P", "pause")];
        for (i, (key, action)) in keys.iter().enumerate() {
            let y = 522.0 + i as f32 * 24.0;
            draw_text(key, px, y, 18.0, TEXT);
            draw_text(action, px + 96.0, y, 18.0, DIM);
        }
    }

    /// Drawn last, over the well: the cleared-row band, then the paused / game-over panel.
    fn draw_overlay(&self) {
        let well_w = W as f32 * CELL;
        for &y in &self.cleared {
            draw_rectangle(0.0, y as f32 * CELL, well_w, CELL, Color::new(1.0, 1.0, 1.0, 0.35 * self.flash / FLASH));
        }
        let lines = if self.over {
            ["Game over".to_string(), format!("Score {}", self.score), "R  new game".to_string()]
        } else if self.paused {
            ["Paused".to_string(), "P  resume".to_string(), "R  new game".to_string()]
        } else {
            return;
        };
        draw_rectangle(0.0, 0.0, well_w, VIEW_H, Color { a: 0.7, ..BG });
        let (w, h) = (260.0, 150.0);
        let (x, y) = ((well_w - w) / 2.0, (VIEW_H - h) / 2.0);
        draw_rectangle(x, y, w, h, BG);
        draw_rectangle_lines(x, y, w, h, 2.0, DIM);
        for (line, (dy, size, color)) in lines.iter().zip([(52.0, 40.0, TEXT), (92.0, 24.0, TEXT), (124.0, 20.0, DIM)]) {
            let tw = measure_text(line, None, size as u16, 1.0).width;
            draw_text(line, (well_w - tw) / 2.0, y + dy, size, color);
        }
    }
}

fn draw_cell(x: f32, y: f32, color: u8) {
    draw_rectangle(x + 1.0, y + 1.0, CELL - 2.0, CELL - 2.0, COLORS[color as usize - 1]);
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
    loop {
        if is_key_pressed(KeyCode::R) && game.can_restart() {
            game = Game::new();
        }
        if is_key_pressed(KeyCode::P) && !game.over {
            game.paused = !game.paused;
        }
        if !game.over && !game.paused {
            game.update();
        }
        game.draw();
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
        g.piece = Piece { cells: vec![(0, 0), (1, 0), (0, 1)], x: 5, y: 2, color: 1 };
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
        g.piece = Piece { cells: vec![(0, 0)], x: 0, y: H - 1, color: 3 };
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
        g.piece = Piece { cells: vec![(0, 0)], x: 0, y: H - 1, color: 3 };
        g.lock();
        assert_eq!(g.lines, 0);
        assert!(g.over);
    }

    #[test]
    fn lock_records_cleared_rows_and_level_up() {
        let mut g = Game::new();
        let bottom = H as usize - 1;
        for row in &mut g.grid[bottom - 1..] {
            *row = [1; W as usize];
            row[0] = 0;
        }
        g.lines = 9;
        g.piece = Piece { cells: vec![(0, 0), (0, 1)], x: 0, y: H - 2, color: 3 };
        g.lock();
        assert_eq!(g.cleared, vec![bottom - 1, bottom]);
        assert_eq!((g.lines, g.flash, g.glow), (11, FLASH, GLOW));

        g.piece = Piece { cells: vec![(0, 0)], x: 0, y: H - 1, color: 3 };
        g.lock();
        assert!(g.cleared.is_empty());
    }

    #[test]
    fn lock_shifts_the_two_upcoming_pieces() {
        let mut g = Game::new();
        let (next, after) = (g.next.cells.clone(), g.after.cells.clone());
        g.piece = Piece { cells: vec![(0, 0)], x: 0, y: H - 1, color: 3 };
        g.lock();
        assert_eq!((g.piece.cells.clone(), g.next.cells.clone()), (next, after));
    }

    #[test]
    fn four_turns_restore_the_piece() {
        let mut g = Game::new();
        let cells = vec![(0, 0), (1, 0), (0, 1), (0, 2)];
        g.piece = Piece { cells: cells.clone(), x: 5, y: 10, color: 1 };
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
        for y in 9..=12 {
            g.grid[y][4] = 1;
            g.grid[y][6] = 1;
        }
        let cells = vec![(0, 0), (0, 1)];
        g.piece = Piece { cells: cells.clone(), x: 5, y: 10, color: 1 };
        g.rotate();
        assert_eq!((&g.piece.cells, g.piece.x, g.piece.y), (&cells, 5, 10));

        // a long bar against the left wall still turns, two cells away from it
        let mut g = Game::new();
        g.piece = Piece { cells: vec![(0, -1), (0, 0), (0, 1), (0, 2)], x: 0, y: 10, color: 1 };
        g.rotate();
        assert_eq!(g.piece.cells, vec![(1, 0), (0, 0), (-1, 0), (-2, 0)]);
        assert_eq!((g.piece.x, g.piece.y), (2, 10));
    }

    #[test]
    fn top_row_piece_can_still_turn() {
        let mut g = Game::new();
        g.piece = Piece { cells: vec![(0, 0), (-1, 0), (1, 0)], x: 5, y: 0, color: 1 };
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
}
