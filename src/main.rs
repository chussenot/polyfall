use macroquad::prelude::*;

const W: i32 = 12;
const H: i32 = 22;
const CELL: f32 = 30.0;
const PANEL: f32 = 200.0;
const VIEW_W: f32 = W as f32 * CELL + PANEL;
const VIEW_H: f32 = H as f32 * CELL;
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
    score: u32,
    lines: u32,
    fall: f32,
    over: bool,
    paused: bool,
}

impl Game {
    fn new() -> Self {
        Game {
            grid: [[0; W as usize]; H as usize],
            piece: random_piece(),
            next: random_piece(),
            score: 0,
            lines: 0,
            fall: 0.0,
            over: false,
            paused: false,
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
        let cleared = H as usize - kept.len();
        self.grid = [[0; W as usize]; H as usize];
        self.grid[cleared..].copy_from_slice(&kept);
        self.lines += cleared as u32;
        self.score += (cleared * cleared) as u32 * 100;
        self.fall = 0.0;
        self.piece = std::mem::replace(&mut self.next, random_piece());
        self.over = !self.fits(&self.piece.cells, self.piece.x, self.piece.y);
    }

    fn level(&self) -> u32 {
        self.lines / 10
    }

    fn update(&mut self) {
        if is_key_pressed(KeyCode::Left) {
            self.shift(-1, 0);
        }
        if is_key_pressed(KeyCode::Right) {
            self.shift(1, 0);
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

    fn draw(&self) {
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
        draw_text(&format!("Score {}", self.score), px, 40.0, 28.0, WHITE);
        draw_text(&format!("Lines {}", self.lines), px, 70.0, 28.0, WHITE);
        draw_text(&format!("Level {}", self.level() + 1), px, 100.0, 28.0, WHITE);
        draw_text("Next", px, 150.0, 28.0, GRAY);
        let left = self.next.cells.iter().map(|c| c.0).min().unwrap();
        for &(cx, cy) in &self.next.cells {
            draw_cell(px + (cx - left) as f32 * CELL, 170.0 + (self.next.y + cy) as f32 * CELL, self.next.color);
        }
        for (i, line) in ["Left/Right  move", "Up  rotate", "Down  soft drop", "Space  hard drop", "R  restart"]
            .iter()
            .enumerate()
        {
            draw_text(line, px, 400.0 + i as f32 * 24.0, 20.0, GRAY);
        }
    }

    /// Drawn last, over the well: game-over state.
    fn draw_overlay(&self) {
        if self.over {
            draw_text("GAME OVER", 60.0, H as f32 * CELL / 2.0, 56.0, WHITE);
            draw_text("press R", 130.0, H as f32 * CELL / 2.0 + 40.0, 32.0, WHITE);
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
        if is_key_pressed(KeyCode::R) {
            game = Game::new();
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
}
