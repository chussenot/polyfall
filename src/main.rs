use macroquad::prelude::*;

const W: i32 = 12;
const H: i32 = 22;
const CELL: f32 = 30.0;
const PANEL: f32 = 200.0;
const BG: Color = Color::new(0.07, 0.08, 0.11, 1.0);
const WELL: Color = Color::new(0.12, 0.13, 0.18, 1.0);
const COLORS: [Color; 6] = [
    Color::new(0.95, 0.55, 0.35, 1.0),
    Color::new(0.45, 0.80, 0.70, 1.0),
    Color::new(0.90, 0.80, 0.40, 1.0),
    Color::new(0.60, 0.55, 0.90, 1.0),
    Color::new(0.90, 0.45, 0.60, 1.0),
    Color::new(0.50, 0.75, 0.95, 1.0),
];

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
    Piece { cells, x: W / 2, y: -top, color: rand::gen_range(1, COLORS.len() as i32 + 1) as u8 }
}

struct Game {
    grid: Grid,
    piece: Piece,
    next: Piece,
    score: u32,
    lines: u32,
    fall: f32,
    over: bool,
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

    fn rotate(&mut self) {
        let turned: Vec<_> = self.piece.cells.iter().map(|&(x, y)| (-y, x)).collect();
        // nudge sideways if the turned shape hits a wall or the stack
        for kick in [0, -1, 1, -2, 2] {
            if self.fits(&turned, self.piece.x + kick, self.piece.y) {
                self.piece.x += kick;
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
        clear_background(BG);
        draw_rectangle(0.0, 0.0, W as f32 * CELL, H as f32 * CELL, WELL);
        for (y, row) in self.grid.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                if c != 0 {
                    draw_cell(x as f32 * CELL, y as f32 * CELL, c);
                }
            }
        }
        let p = &self.piece;
        for &(cx, cy) in &p.cells {
            draw_cell((p.x + cx) as f32 * CELL, (p.y + cy) as f32 * CELL, p.color);
        }

        let px = W as f32 * CELL + 20.0;
        draw_text(&format!("Score {}", self.score), px, 40.0, 28.0, WHITE);
        draw_text(&format!("Lines {}", self.lines), px, 70.0, 28.0, WHITE);
        draw_text(&format!("Level {}", self.level() + 1), px, 100.0, 28.0, WHITE);
        draw_text("Next", px, 150.0, 28.0, GRAY);
        for &(cx, cy) in &self.next.cells {
            draw_cell(px + 70.0 + cx as f32 * CELL, 170.0 + (self.next.y + cy) as f32 * CELL, self.next.color);
        }
        for (i, line) in ["Left/Right  move", "Up  rotate", "Down  soft drop", "Space  hard drop", "R  restart"]
            .iter()
            .enumerate()
        {
            draw_text(line, px, 400.0 + i as f32 * 24.0, 20.0, GRAY);
        }
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
        window_width: (W as f32 * CELL + PANEL) as i32,
        window_height: (H as f32 * CELL) as i32,
        window_resizable: false,
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
        if !game.over {
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
    }

    #[test]
    fn rotation_never_leaves_the_well() {
        let mut g = Game::new();
        for _ in 0..200 {
            g.piece = random_piece();
            while g.shift(-1, 0) {}
            for _ in 0..4 {
                g.rotate();
                assert!(g.fits(&g.piece.cells, g.piece.x, g.piece.y));
            }
        }
    }
}
