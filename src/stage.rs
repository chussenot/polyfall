//! Staging: function keys that set up a situation random pieces would take minutes to produce.
//!
//! Compiled only with `--features stage`; the game as shipped has none of this. It exists for
//! the sound bench in `tools/sound`, which presses these keys and records what is heard, and
//! for trying a sound or an effect by hand: `cargo run --features stage`, then F4 and Space.
//!
//! A stage only arranges the well, the falling piece and the row count. What happens next is
//! ordinary play through the ordinary keys, so nothing here can be mistaken for a game rule,
//! and a stage makes no sound of its own.

use crate::{Game, Piece, H, W};
use macroquad::prelude::{is_key_pressed, KeyCode};

const WIDE: usize = W as usize;
const TALL: usize = H as usize;

/// Applies the stage whose key was pressed this frame, if any. `main` calls it only during play.
///
/// | Key | Stage |
/// | --- | --- |
/// | F1 to F5 | Rows about to clear: a hard drop removes 1 to 5 of them. |
/// | F6 | The next row cleared raises the level. |
/// | F7 | A well filled to the brim: the next lock ends the game. |
/// | F8 | An empty well, the piece back at the top. |
pub fn keys(game: &mut Game) {
    let rows = [
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
    ];
    for (n, key) in rows.into_iter().enumerate() {
        if is_key_pressed(key) {
            rows_to_clear(game, n + 1);
        }
    }
    if is_key_pressed(KeyCode::F6) {
        next_row_levels_up(game);
    }
    if is_key_pressed(KeyCode::F7) {
        brim(game);
    }
    if is_key_pressed(KeyCode::F8) {
        empty(game);
    }
}

/// A well whose bottom `n` rows lack only column 0, with loose blocks above them, and a
/// straight piece over column 0. A hard drop clears exactly `n` rows and brings the rest down.
fn rows_to_clear(game: &mut Game, n: usize) {
    game.grid = [[0; WIDE]; TALL];
    for (y, row) in game.grid.iter_mut().enumerate().skip(TALL - n) {
        for (x, cell) in row.iter_mut().enumerate().skip(1) {
            *cell = ((x + y) % 3 + 1) as u8;
        }
    }
    // loose blocks on top, so there is something to come down; none in column 0
    for (x, height) in [
        (1, 3),
        (2, 1),
        (4, 2),
        (5, 4),
        (7, 1),
        (8, 3),
        (10, 2),
        (11, 1),
    ] {
        for k in 0..height {
            game.grid[TALL - n - 1 - k][x] = ((x + k) % 3 + 1) as u8;
        }
    }
    let cells = n.max(3); // a real piece has 3 to 5 cells; the ones above the rows stay behind
    game.piece = Piece {
        cells: (0..cells as i32).map(|i| (0, i)).collect(),
        x: 0,
        y: 0,
        color: cells as u8 - 2,
    };
    game.fall = 0.0;
}

/// Sets the row count one short of the next level.
fn next_row_levels_up(game: &mut Game) {
    game.lines = game.level() * 10 + 9;
}

/// A well filled up to row 5, with a hole in every row so that none is full, and column 6
/// blocked to the top. The falling piece still has room on the left. The piece after it has
/// none, whatever its shape: every piece appears with a cell in that column, in rows 0 to 4.
fn brim(game: &mut Game) {
    game.grid = [[0; WIDE]; TALL];
    for (y, row) in game.grid.iter_mut().enumerate().skip(5) {
        for (x, cell) in row.iter_mut().enumerate() {
            if x != (y * 5) % WIDE {
                *cell = ((x + y) % 3 + 1) as u8;
            }
        }
    }
    for row in game.grid.iter_mut().take(5) {
        row[WIDE / 2] = 2;
    }
    game.piece = Piece {
        cells: vec![(0, 0), (1, 0), (2, 0)],
        x: 1,
        y: 0,
        color: 1,
    };
    game.fall = 0.0;
}

/// An empty well, with the falling piece back on its spawn row. Pressed every few seconds it
/// keeps anything from landing: for hearing the theme alone.
fn empty(game: &mut Game) {
    game.grid = [[0; WIDE]; TALL];
    let top = game.piece.cells.iter().map(|c| c.1).min().unwrap();
    game.piece.y = -top;
    game.fall = 0.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Event;

    fn fits(g: &Game) -> bool {
        g.fits(&g.piece.cells, g.piece.x, g.piece.y)
    }

    #[test]
    fn a_hard_drop_on_staged_rows_clears_exactly_that_many() {
        for n in 1..=5 {
            let mut g = Game::new();
            g.grid[3][4] = 1; // whatever was there goes
            rows_to_clear(&mut g, n);
            assert!(fits(&g));
            assert!((3..=5).contains(&g.piece.cells.len()));
            g.hard_drop();
            assert_eq!((g.lines, g.score), (n as u32, (n * n) as u32 * 100));
            // the loose blocks are still there, lower down, and the game goes on
            assert!(g.grid.iter().flatten().any(|&c| c != 0));
            assert!(!g.over);
        }
    }

    #[test]
    fn the_row_after_f6_raises_the_level() {
        let mut g = Game::new();
        g.lines = 23;
        next_row_levels_up(&mut g);
        assert_eq!(g.lines, 29);
        rows_to_clear(&mut g, 1);
        g.hard_drop();
        assert!(g.events.contains(&Event::LevelUp { level: 3 }));
    }

    #[test]
    fn the_lock_after_the_brim_ends_the_game_whatever_piece_comes_next() {
        for _ in 0..300 {
            let mut g = Game::new();
            brim(&mut g);
            assert!(fits(&g));
            g.hard_drop();
            assert!(g.over);
            assert_eq!(g.lines, 0); // no row was full
        }
    }

    #[test]
    fn an_empty_well_puts_the_piece_back_at_the_top() {
        let mut g = Game::new();
        g.grid[10][3] = 1;
        let spawn = g.piece.y;
        for _ in 0..6 {
            g.descend(false);
        }
        empty(&mut g);
        assert!(g.grid.iter().flatten().all(|&c| c == 0));
        assert_eq!(g.piece.y, spawn);
        assert!(fits(&g));
    }
}
