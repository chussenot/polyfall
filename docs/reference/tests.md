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

This page lists what the tests prove and what they leave out. Read it before
you change game logic, to know whether a test will catch a mistake.

## How the tests work

The 13 tests are in the `tests` module at the end of `src/main.rs`. Run them
with `mise run test`. They open no window. Each test builds a `Game`, sets
fields by hand to create a situation, calls one function and checks the result.

Functions that read the keyboard or draw cannot be tested this way.
[Development](../development.md#tests-run-without-a-window) explains the rule.

## What each test proves

### Pieces

| Test | Function tested | What it proves |
| --- | --- | --- |
| `random_pieces_spawn_inside_the_well` | `random_piece`, `Game::fits` | Each of 1000 new pieces has 3 to 5 cells and fits in an empty well where it appears. |
| `colour_follows_cell_count` | `random_piece` | For each of 1000 new pieces, `color` is the cell count minus 2. |
| `landing_row_stops_on_the_stack_or_the_floor` | `Game::landing_y` | The landing row is just above a locked cell under the piece, or on the floor when the column is free. A piece already on the floor gets its own row. |

### Locking

| Test | Function tested | What it proves |
| --- | --- | --- |
| `full_row_clears_and_stack_drops` | `Game::lock` | Filling the bottom row gives `lines` 1 and `score` 100. A locked cell from the row above ends on the bottom row. The game is not over. |
| `lock_records_cleared_rows_and_level_up` | `Game::lock` | Removing two rows when `lines` is 9 records both row indices in `cleared`, gives `lines` 11 and starts `flash` and `glow`. The next lock, which removes nothing, empties `cleared`. |
| `lock_shifts_the_two_upcoming_pieces` | `Game::lock` | After a lock, `piece` has the cells `next` had and `next` has the cells `after` had. |
| `blocked_spawn_area_ends_the_game` | `Game::lock` | With rows 0 to 4 filled except column 0, a lock removes nothing and sets `over`. |

### Rotation

| Test | Function tested | What it proves |
| --- | --- | --- |
| `four_turns_restore_the_piece` | `Game::rotate` | One turn changes the cells of a four-cell piece. Four turns in open space restore its cells and position. |
| `rotation_kicks_off_walls_but_not_over_the_stack` | `Game::rotate` | A two-cell piece between two columns of locked cells does not turn. A four-cell bar against the left wall turns and ends two columns to the right. |
| `top_row_piece_can_still_turn` | `Game::rotate` | A three-cell bar on row 0 turns and moves down one row. |

[Rotation and kicks](rotation-and-kicks.md#three-examples) works through these
cases.

### Input and timing

| Test | Function tested | What it proves |
| --- | --- | --- |
| `restart_only_when_over_or_paused` | `Game::can_restart` | False for a new game, true when paused, true when over. |
| `held_key_repeats_after_a_delay` | `Game::repeats` | No repeat move before 0.17 s. One when the hold time passes 0.17 s. None between 0.18 s and 0.20 s. Three for one step from 0.0 s to 0.28 s. |
| `idle_play_pauses_itself` | `Game::tick_idle` | 5 s without a key does not pause. A key restarts the count. 8 s without a key pauses. The count stays at zero during a pause. A game that is over is never paused. |

## What has no test

| Area | Behaviour with no test |
| --- | --- |
| `Game::update` | The whole function: reading keys, the move on a press, applying repeat moves, both keys held, hard drop, soft drop, falling, and the lock when the piece cannot move down. |
| `main` | The order of a frame, the P toggle, P being ignored when the game is over, and the seeding of the random generator. |
| Rendering | `conf`, `draw_cell` and the `Game::draw` family, including the countdown of `flash` and `glow`. |
| `Game::shift` | No test calls it. |
| Fall speed | The formula `(0.6 - 0.05 * level).max(0.08)` and the soft-drop value `0.04`. |
| `Game::level` | Not checked directly. One test checks only that `glow` starts when `lines` goes from 9 to 11. |
| Scoring | Only the value for one row, 100. The test that removes two rows does not check `score`. |
| `Game::lock` | That `fall` returns to `0.0`, that `after` gets a new piece, and that the piece that ends the game stays in place. |
| `Game::rotate` | A one-column kick, a two-column kick off the right wall, a refusal at the floor, and the refusal for a piece that would reach two rows above row 0. |
| Piece generation | That a piece is connected, and how often each shape and size appears. |

The tests do not seed the random generator. It starts from its default state.
