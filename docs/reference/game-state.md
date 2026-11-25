---
title: Game state
description: Every field of the Game and Piece structs, with its type, its meaning and the functions that write and read it.
source:
  - Game
  - Piece
  - Grid
  - Game::new
  - Game::fits
  - Game::level
  - Game::can_restart
  - Game::landing_y
---

# Game state

This page is a lookup table for the data of the game. Use it to find what a
field means and which functions touch it before you change one.

## The idea

One value of `struct Game` holds a whole game. `main` creates it with
`Game::new()` and owns it. A restart replaces it with a new one. Nothing else is
stored, apart from the state of macroquad's random generator.

A `Game` contains three values of `struct Piece`: the falling piece and the two
upcoming pieces.

## `Game`

`Game::new` sets every field to the initial value below. "Written by" lists the
functions that change the field afterwards.

| Field | Type | Initial value | Meaning | Written by | Read by |
| --- | --- | --- | --- | --- | --- |
| `grid` | `Grid`, which is `[[u8; 12]; 22]` | all `0` | The well, as `grid[row][column]`. `0` is an empty cell. 1, 2 or 3 is a locked cell and its colour number. | `lock` | `fits`, `rotate`, `lock`, `draw_well` |
| `piece` | `Piece` | `random_piece()` | The falling piece. | `shift`, `rotate`, `lock` | `shift`, `landing_y`, `rotate`, `lock`, `draw_well` |
| `next` | `Piece` | `random_piece()` | The first upcoming piece. | `lock` | `lock`, `draw_panel` |
| `after` | `Piece` | `random_piece()` | The second upcoming piece. It follows `next`. | `lock` | `lock`, `draw_panel` |
| `score` | `u32` | `0` | Points. | `lock` | `draw_panel`, `draw_overlay` |
| `lines` | `u32` | `0` | Rows removed since the game started. | `lock` | `level`, `draw_panel` |
| `fall` | `f32` | `0.0` | Seconds since the piece last moved down by itself, or since the last lock. | `update`, `lock` | `update` |
| `held` | `f32` | `0.0` | Seconds Left or Right has been held, for key repeat. A new press sets it to `0.0`; a release does not. | `update` | `update` |
| `idle` | `f32` | `0.0` | Seconds of play with no key held, for the automatic pause. | `tick_idle` | `tick_idle` |
| `over` | `bool` | `false` | The game has ended. | `lock` | `main`, `can_restart`, `tick_idle`, `draw_overlay` |
| `paused` | `bool` | `false` | The game is paused, by P or by the automatic pause. | `main`, `tick_idle` | `main`, `can_restart`, `tick_idle`, `draw_overlay` |
| `cleared` | `Vec<usize>` | empty | Indices of the rows the last lock removed, as they were before the rows above moved down. Every lock replaces it, so it is empty after a lock that removes nothing. | `lock` | `lock`, `draw_overlay` |
| `flash` | `f32` | `0.0` | Seconds left on the band over removed rows. | `lock`, `draw` | `draw`, `draw_overlay` |
| `glow` | `f32` | `0.0` | Seconds left on the level highlight. | `lock`, `draw` | `draw`, `draw_panel` |

## `Piece`

| Field | Type | Meaning | Written by |
| --- | --- | --- | --- |
| `cells` | `Vec<(i32, i32)>` | The cells of the piece, each an offset `(column, row)` from `(x, y)`. Always contains `(0, 0)`, the cell the piece turns around. A piece from `random_piece` has 3 to 5 cells. | `random_piece`, `rotate` |
| `x` | `i32` | Column of the cell `(0, 0)`. A new piece has `W / 2`, which is 6. | `random_piece`, `shift`, `rotate` |
| `y` | `i32` | Row of the cell `(0, 0)`. A new piece has the value that puts its topmost cell on row 0. | `random_piece`, `shift`, `rotate` |
| `color` | `u8` | Colour number: the cell count minus 2, so 1, 2 or 3. The colour drawn is `COLORS[color - 1]`. | `random_piece` |

`next` and `after` have a position too, but it is not used while they wait. The
side panel centres them in their slots.

## Values computed on demand

These are not stored.

| Function | Returns | Computed from |
| --- | --- | --- |
| `Game::level()` | `lines / 10` | `lines` |
| `Game::can_restart()` | `over \|\| paused` | `over`, `paused` |
| `Game::landing_y()` | The row the falling piece reaches if it moves straight down until it stops. | `piece`, `grid` |
| `Game::fits(cells, x, y)` | Whether every cell is inside the well and on an empty cell. | `grid` |

## Rules the code relies on

Keep these true when you change the code.

- **The falling piece always fits where it is**, as long as the game is not
  over. `shift` and `rotate` move it only after `fits` succeeds. `lock` relies
  on this: it writes the cells into `grid` without checking the indices.
- **`color` is never `0` for a piece.** Drawing code indexes
  `COLORS[color as usize - 1]`.
- **`over` and `paused` are never both true in normal play.** P is ignored
  when the game is over, and `tick_idle` does not pause a game that is over.

The moves between playing, paused and over are on
[Controls and timing](controls-and-timing.md#pause-and-restart).
