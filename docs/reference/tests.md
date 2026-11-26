---
title: Tests
description: What each of the 27 unit tests in src/main.rs and src/fx.rs proves, and which behaviour has no test.
source:
  - tests::random_pieces_spawn_inside_the_well
  - tests::landing_row_stops_on_the_stack_or_the_floor
  - tests::colour_follows_cell_count
  - tests::full_row_clears_and_stack_drops
  - tests::blocked_spawn_area_ends_the_game
  - tests::lock_reports_cleared_rows_and_level_up
  - tests::score_is_rows_squared_times_100
  - tests::moves_and_turns_are_reported_only_when_they_happen
  - tests::hard_drop_lands_where_the_outline_was_and_reports_the_distance
  - tests::the_end_of_the_game_is_reported_once_after_the_last_lock
  - src/fx.rs
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

There are 27 tests: 17 for the game in the `tests` module at the end of
`src/main.rs`, and 10 for the effects in the `tests` module at the end of
`src/fx.rs`. Run them with `mise run test`. They open no window. Each test builds a `Game`, sets
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
| `lock_reports_cleared_rows_and_level_up` | `Game::lock` | Removing two rows when `lines` is 9 gives `lines` 11 and `score` 400, and pushes exactly `Locked` (with the cells, both row indices, the well before removal and 400 points) then `LevelUp` to level 1. The next lock, which removes nothing, pushes one `Locked` with no rows and 0 points. |
| `score_is_rows_squared_times_100` | `Game::lock` | Removing 1, 2, 3, 4 and 5 rows at once gives 100, 400, 900, 1600 and 2500 points, the same count in `lines`, and an empty well. |
| `hard_drop_lands_where_the_outline_was_and_reports_the_distance` | `Game::hard_drop` | The piece is locked on the row `landing_y` gave. `HardDropped` carries the distance and `Locked` follows. `fall` is `0.0`. |
| `the_end_of_the_game_is_reported_once_after_the_last_lock` | `Game::lock` | A lock that ends the game pushes `Locked` then `GameOver`. |
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

### Events from moves

| Test | Function tested | What it proves |
| --- | --- | --- |
| `moves_and_turns_are_reported_only_when_they_happen` | `Game::shift`, `Game::rotate` | A move refused by the wall pushes nothing. A sideways move, a move down and a kicked turn push `Moved` and `Rotated` with the right distances, in order. |

### Input and timing

| Test | Function tested | What it proves |
| --- | --- | --- |
| `restart_only_when_over_or_paused` | `Game::can_restart` | False for a new game, true when paused, true when over. |
| `held_key_repeats_after_a_delay` | `Game::repeats` | No repeat move before 0.17 s. One when the hold time passes 0.17 s. None between 0.18 s and 0.20 s. Three for one step from 0.0 s to 0.28 s. |
| `idle_play_pauses_itself` | `Game::tick_idle` | 5 s without a key does not pause. A key restarts the count. 8 s without a key pauses. The count stays at zero during a pause. A game that is over is never paused. |

### Effects

These tests are in `src/fx.rs`. They feed events to an `Fx` value, advance it
with `Fx::tick` and check what it reports. They check behaviour, not exact
tuning values. [Effects](effects.md) describes what they test.

| Test | What it proves |
| --- | --- |
| `rows_above_a_clear_start_where_they_were_and_end_in_place` | After two rows are removed, a cell above them is first drawn two rows up, and at its real place once the collapse has finished. |
| `rows_between_two_cleared_rows_come_down_by_one` | With rows removed above and below it, a row comes down by one and the rows above by two. A cell of the piece that survived is followed to its new row. |
| `every_cleared_cell_breaks_up_exactly_once` | For 1 to 5 rows, each cell throws its fragments once, the count never passes `MAX_SPARKS`, and nothing is left afterwards. |
| `a_bigger_clear_is_a_bigger_event` | From 1 to 4 rows, each step gives more fragments and a harder knock. |
| `effects_do_not_depend_on_the_frame_rate` | 0.1 s played in 3 steps and in 24 steps gives the same piece offset, shown score, knock, fragment count and fragment positions. |
| `the_drawn_piece_starts_where_it_was_and_catches_up` | After a move the drawn piece is one cell back. A kick is added to that. After 0.3 s the offset, the angle and the brightening are gone. |
| `only_a_hard_drop_leaves_a_trail_and_knocks_the_well` | A piece that settled has no trail, fragments or knock. A hard drop has all three and a brighter flash, and counts both floor contacts. The next lock is an ordinary one again. |
| `the_shown_score_counts_up_to_the_real_one_and_never_past_it` | The shown score starts at the old value, never goes down or past the real one, and reaches it. The gain label then goes away. |
| `the_end_is_held_then_dimmed_from_the_top_down` | Nothing dims during the hold. Then the top row is darker than the bottom row before the box appears, and all end fully shown. With no event, the overlay is fully shown. |
| `levels_raise_the_energy_up_to_a_ceiling` | `LevelUp` restarts the level effect and sets the energy to a tenth of the level, at most 1. |

## What has no test

| Area | Behaviour with no test |
| --- | --- |
| `Game::update` | The whole function: reading keys, the move on a press, applying repeat moves, both keys held, hard drop, soft drop, falling, and the lock when the piece cannot move down. |
| `main` | The order of a frame, the P toggle, P being ignored when the game is over, and the seeding of the random generator. |
| Rendering | `conf`, `draw_piece`, the `Game::draw` family and the three `Fx::draw_*` functions: what is actually drawn. |
| Effects in `main` | That `Fx::tick` is skipped during a pause and that a restart replaces `Fx`. |
| Fall speed | The formula `(0.6 - 0.05 * level).max(0.08)` and the soft-drop value `0.04`. |
| `Game::level` | Not checked directly. One test checks only that `LevelUp` is pushed when `lines` goes from 9 to 11. |
| `Game::lock` | That `after` gets a new piece, and that the piece that ends the game stays in place. |
| `Game::rotate` | A one-column kick, a two-column kick off the right wall, a refusal at the floor, and the refusal for a piece that would reach two rows above row 0. |
| Piece generation | That a piece is connected, and how often each shape and size appears. |

The tests do not seed the random generator. It starts from its default state.
