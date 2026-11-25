---
title: Tests
description: What each of the 13 unit tests in src/main.rs proves, and which behaviour has no test.
source:
  - tests::random_pieces_spawn_inside_the_well
  - tests::landing_row_stops_on_the_stack_or_the_floor
  - tests::colour_follows_cell_count
  - tests::full_row_clears_and_stack_drops
  - tests::blocked_spawn_area_ends_the_game
  - tests::lock_records_cleared_rows_and_level_up
  - tests::lock_shifts_the_two_upcoming_pieces
  - tests::four_turns_restore_the_piece
  - tests::rotation_kicks_off_walls_but_not_over_the_stack
  - tests::top_row_piece_can_still_turn
  - tests::restart_only_when_over_or_paused
  - tests::held_key_repeats_after_a_delay
  - tests::idle_play_pauses_itself
---

# Tests

The tests are in the `tests` module at the end of `src/main.rs`, under
`#[cfg(test)]`. Run them with `cargo test`. They open no window: they call
`Game` methods and `random_piece` directly and set `Game` fields by hand.

The tests never call `rand::srand`, so they use the random generator in its
initial state.

## What each test proves

| Test | Function under test | What it asserts |
| --- | --- | --- |
| `random_pieces_spawn_inside_the_well` | `random_piece`, `Game::fits` | 1000 generated pieces each have 3 to 5 cells and fit in an empty well at their spawn position. |
| `colour_follows_cell_count` | `random_piece` | For 1000 pieces, `color` equals the cell count minus 2. |
| `landing_row_stops_on_the_stack_or_the_floor` | `Game::landing_y` | A piece lands on a locked block under one of its cells (row 14 above a block at row 15); in a free column it lands on the floor (`H - 2` for a two-row shape); a piece already resting returns its own row. |
| `full_row_clears_and_stack_drops` | `Game::lock` | Completing the bottom row gives `lines == 1` and `score == 100`; a block from the row above ends on the bottom row; the bottom row then holds exactly one block; the game is not over. |
| `blocked_spawn_area_ends_the_game` | `Game::lock` | With rows 0 to 4 filled except column 0, a lock clears nothing and sets `over`. |
| `lock_records_cleared_rows_and_level_up` | `Game::lock` | Clearing two rows from `lines == 9` records both row indices in `cleared`, gives `lines == 11`, `flash == FLASH` and `glow == GLOW`. A following lock that clears nothing empties `cleared`. |
| `lock_shifts_the_two_upcoming_pieces` | `Game::lock` | After a lock, `piece` has the cells `next` had and `next` has the cells `after` had. |
| `four_turns_restore_the_piece` | `Game::rotate` | One turn changes the cells of a four-cell shape; four turns in open space restore cells and position. |
| `rotation_kicks_off_walls_but_not_over_the_stack` | `Game::rotate` | A two-cell piece in a one-column shaft between two columns of blocks does not rotate. A four-cell bar against the left wall rotates and ends two columns from its start. |
| `top_row_piece_can_still_turn` | `Game::rotate` | A three-cell horizontal piece on row 0 rotates and moves down one row. |
| `restart_only_when_over_or_paused` | `Game::can_restart` | False for a new game, true when `paused`, true when `over`. |
| `held_key_repeats_after_a_delay` | `Game::repeats` | No repeat before 0.17 s; one when crossing it; none between 0.18 and 0.20; three for a single step from 0.0 to 0.28. |
| `idle_play_pauses_itself` | `Game::tick_idle` | 5 s idle does not pause; a key restarts the count; 8 s in total pauses; the count restarts while paused; a game that is over never pauses. |

## What is not tested

No test covers the following. This list is the difference between the code and
the assertions above.

| Area | Untested behaviour |
| --- | --- |
| `Game::update` | The whole function: reading keys, the first move on a press, applying repeats, both keys held, hard drop, soft-drop interval, gravity, locking when a gravity step fails. |
| Speed curve | The interval formula `(0.6 - 0.05 * level).max(0.08)` and the `0.04` soft-drop value. |
| `Game::level` | Not asserted directly. Only its effect on `glow` at the 9 to 11 line step is checked. |
| Scoring | Only the one-row value (100). The two-row lock in `lock_records_cleared_rows_and_level_up` does not check `score`. |
| `Game::lock` | `fall` being reset; that `after` receives a new piece; that the game-over piece stays in place. |
| `Game::rotate` | A two-column kick off the right wall; a one-column kick; refusal at the floor; the documented limit for shapes reaching two or more rows above row 0. |
| `Game::shift` | Not called by any test. |
| `main` | The order R, P, `tick_idle`, `update`, `draw`; the P toggle; that P is ignored when over; seeding. |
| Rendering | `conf`, `Game::draw`, `draw_well`, `draw_panel`, `draw_overlay`, `draw_cell`, including the decrease of `flash` and `glow`. |
| Piece generation | That shapes are connected; the distribution of shapes and sizes. |
