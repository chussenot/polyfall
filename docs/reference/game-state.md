---
title: Game state
description: Every field of the Game and Piece structs - type, meaning, which functions write it and which read it.
source:
  - Game
  - Piece
  - Grid
  - Game::new
  - Game::level
  - Game::can_restart
  - Game::landing_y
---

# Game state

The whole state of a game is one value of `struct Game`, created by
`Game::new()` and owned by `main`. Restarting replaces the value. Nothing is
stored outside it, apart from the state of macroquad's random generator.

"Writes" below lists functions that assign the field after construction;
`Game::new` initialises every field and is not repeated in each row. Test code
also assigns fields directly and is not listed.

## `Game`

| Field | Type | Initial value | Meaning | Written by | Read by |
| --- | --- | --- | --- | --- | --- |
| `grid` | `Grid` (`[[u8; 12]; 22]`) | all `0` | Locked cells, `grid[row][column]`. `0` is empty; other values are 1-based indices into `COLORS`. | `lock` | `fits`, `rotate`, `lock`, `draw_well` |
| `piece` | `Piece` | `random_piece()` | The falling piece. | `shift`, `rotate`, `lock` | `shift`, `landing_y`, `rotate`, `lock`, `draw_well` |
| `next` | `Piece` | `random_piece()` | The piece that falls after the current one. | `lock` | `lock`, `draw_panel` |
| `after` | `Piece` | `random_piece()` | The piece that follows `next`. | `lock` | `lock`, `draw_panel` |
| `score` | `u32` | `0` | Points. Grows by `cleared * cleared * 100` per lock. | `lock` | `draw_panel`, `draw_overlay` |
| `lines` | `u32` | `0` | Total rows cleared. | `lock` | `level`, `draw_panel` |
| `fall` | `f32` | `0.0` | Seconds since the last gravity step or lock. | `update`, `lock` | `update` |
| `held` | `f32` | `0.0` | Seconds Left or Right has been held, for key repeat. Reset on a new press, not on release. | `update` | `update` |
| `idle` | `f32` | `0.0` | Seconds of play with no key held. | `tick_idle` | `tick_idle` |
| `over` | `bool` | `false` | The game has ended. Only `lock` sets it. | `lock` | `main`, `can_restart`, `tick_idle`, `draw_overlay` |
| `paused` | `bool` | `false` | The game is paused, by P or by idleness. | `main`, `tick_idle` | `main`, `can_restart`, `tick_idle`, `draw_overlay` |
| `cleared` | `Vec<usize>` | empty | Row indices removed by the last lock, as they were before the collapse. Replaced at every lock, so it is empty after a lock that clears nothing. | `lock` | `lock`, `draw_overlay` |
| `flash` | `f32` | `0.0` | Seconds left on the cleared-row band. | `lock`, `draw` | `draw_overlay` |
| `glow` | `f32` | `0.0` | Seconds left on the level highlight. | `lock`, `draw` | `draw_panel` |

## `Piece`

| Field | Type | Meaning | Written by |
| --- | --- | --- | --- |
| `cells` | `Vec<(i32, i32)>` | Cell offsets `(column, row)` from `(x, y)`. Always contains `(0, 0)`, the rotation pivot. 3 to 5 entries for generated pieces. | `random_piece`, `rotate` |
| `x` | `i32` | Column of the pivot. `W / 2` (6) at creation. | `random_piece`, `shift`, `rotate` |
| `y` | `i32` | Row of the pivot. At creation, minus the smallest row offset, so the top cell is on row 0. | `random_piece`, `shift`, `rotate` |
| `color` | `u8` | 1-based index into `COLORS`: cell count minus 2. Never `0` for a generated piece; `0` means empty in the grid. | `random_piece` |

`next` and `after` carry a spawn position from creation but are drawn only in
the preview slots, where the position is ignored.

## Derived values

These are computed on demand and not stored.

| Function | Returns | From |
| --- | --- | --- |
| `Game::level()` | `lines / 10` | `lines` |
| `Game::can_restart()` | `over \|\| paused` | `over`, `paused` |
| `Game::landing_y()` | Row the falling piece would rest on if dropped straight down | `piece`, `grid` |
| `Game::fits(cells, x, y)` | Whether every cell is inside the well and on an empty grid cell | `grid` |

## Invariants the code relies on

- The falling piece always fits at its position while the game is not over.
  `shift` and `rotate` change it only after `fits` succeeds. `lock` depends on
  this: it indexes `grid` with the piece cells without a bounds check.
- `piece.color`, `next.color` and `after.color` are at least 1. `draw_cell` and
  `draw_panel` compute `COLORS[color as usize - 1]`.
- `over` and `paused` are never both true through normal play: P is ignored
  when `over` is true, and `tick_idle` keeps `idle` at `0.0` when `over` is
  true.

For the transitions between playing, paused and over, see
[Controls and timing](controls-and-timing.md).
