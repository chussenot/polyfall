---
title: Tests
description: What each of the 58 unit tests in src/main.rs, src/fx.rs, src/sound.rs, src/best.rs and src/stage.rs proves, and which behaviour has no test.
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
  - tests::a_key_press_that_cannot_move_is_reported
  - tests::only_a_row_fallen_with_down_held_is_a_soft_drop
  - src/fx.rs
  - src/sound.rs
  - src/stage.rs
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

There are 58 tests: 22 for the game in the `tests` module at the end of
`src/main.rs`, 14 for the effects in the `tests` module at the end of
`src/fx.rs`, 16 for the sound in the `tests` module at the end of
`src/sound.rs`, 2 for the best score in `src/best.rs`, and 4 for the staging
keys in `src/stage.rs`. Run them with `mise run test`, which is
`cargo test --all-features`: plain `cargo test` leaves the staging keys out and
runs 54. They open no window and play no
sound. Each test builds a `Game`, sets
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
| `a_resting_piece_locks_after_the_lock_delay_and_not_before` | `Game::gravity` | With Down held or not, a piece on the floor is still free 0.01 s before `LOCK_DELAY` and locked after it. The next piece starts with no time on the ground and no resets used. |
| `a_move_on_the_ground_restarts_the_delay_a_limited_number_of_times` | `Game::gravity` | `LOCK_RESETS` moves, each just before the lock, keep the piece free. The next move does not: it locks. |
| `a_piece_in_the_air_falls_and_does_not_count_toward_the_lock` | `Game::gravity` | A piece with room below gathers no time on the ground and falls one row after the fall interval. |

### Best score

| Test | What it proves |
| --- | --- |
| `a_saved_best_is_read_back` | No file reads as 0. A saved best is read back, and a lower one written later replaces it. The directory is created. |
| `an_unreadable_file_counts_as_no_best` | A file that holds no number reads as 0, so does no file at all, and saving with no file does nothing. |

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

| `a_key_press_that_cannot_move_is_reported` | `Game::sidestep`, `Game::rotate` | A press against the wall pushes `Blocked` and nothing else. A press with room pushes `Moved`. A turn with no room pushes `Blocked`. |
| `only_a_row_fallen_with_down_held_is_a_soft_drop` | `Game::descend` | A row fallen by gravity pushes `Moved` alone. A row fallen with Down held pushes `Moved`, then `SoftDropped`. A piece on the floor locks instead, and a lock is not a soft drop. |

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
| `a_lock_during_a_clear_hurries_it_without_a_jump` | A lock 0.06, 0.13 or 0.22 s into a clear leaves the stack drawn exactly where it was, draws the new piece where it landed, brings the stack to its real place within four frames without ever moving it back up, and still lets every cleared cell break up once. |
| `a_clear_during_a_clear_takes_over_without_a_jump` | A second clear 0.1 or 0.22 s into the first replaces it, draws the stack exactly where it was, and then plays as a normal clear. |
| `irregular_frames_end_in_the_same_place` | 0.3 s with a hard drop, a three-row clear and a lock in the middle of it gives the same stack position, shown score, knock and fragment count in even 20 ms frames and in frames from 1 to 60 ms. |
| `nothing_is_left_running_after_a_long_frame` | After one 5 s frame every effect has ended: no clear, lock, trail, fragment, offset, knock or label is left. |
| `every_cleared_cell_breaks_up_exactly_once` | For 1 to 5 rows, each cell throws its fragments once, the count never passes `MAX_SPARKS`, and nothing is left afterwards. |
| `a_bigger_clear_is_a_bigger_event` | From 1 to 4 rows, each step gives more fragments and a harder knock. |
| `effects_do_not_depend_on_the_frame_rate` | 0.1 s played in 3 steps and in 24 steps gives the same piece offset, shown score, knock, fragment count and fragment positions. |
| `the_drawn_piece_starts_where_it_was_and_catches_up` | After a move the drawn piece is one cell back. A kick is added to that. After 0.3 s the offset, the angle and the brightening are gone. |
| `only_a_hard_drop_leaves_a_trail_and_knocks_the_well` | A piece that settled has no trail, fragments or knock. A hard drop has all three and a brighter flash, and counts both floor contacts. The next lock is an ordinary one again. |
| `the_shown_score_counts_up_to_the_real_one_and_never_past_it` | The shown score starts at the old value, never goes down or past the real one, and reaches it. The gain label then goes away. |
| `the_end_is_held_then_dimmed_from_the_top_down` | Nothing dims during the hold. Then the top row is darker than the bottom row before the box appears, and all end fully shown. With no event, the overlay is fully shown. |
| `levels_raise_the_energy_up_to_a_ceiling` | `LevelUp` restarts the level effect and sets the energy to a tenth of the level, at most 1. |

### Sound

These tests are in `src/sound.rs`. They feed events to a `Desk`, advance it with
`Desk::frame` and check the orders it writes. No sound card is involved. They
check behaviour, not exact levels. [Sound](sound.md) describes what they test.

| Test | What it proves |
| --- | --- |
| `the_theme_starts_with_the_desk` | A new desk has one order waiting: loop the theme. An idle frame adds none. |
| `every_action_of_the_player_is_answered` | A move left, a move right, a turn, a kicked turn, a blocked press and a lock each play their own cue and no other. A soft-drop row starts the loop. |
| `gravity_makes_no_sound` | Twenty rows fallen without Down held order nothing. |
| `a_held_key_ticks_no_faster_than_the_mixer_can_play` | For a key held one second, no two ticks are closer than `MOVE_GAP`, about every other move is heard, the press is at full level and the repeats are quieter. Two presses a fifth of a second apart are both at full level. |
| `the_soft_drop_runs_while_rows_fall_and_stops_when_they_stop` | Ten rows start one loop and do not stop it. A frame of a third of a second does not stop it if a row fell in that frame. It stops once when the rows stop, and at once when the piece locks. The next row starts it again. |
| `a_longer_fall_lands_harder_and_a_settled_piece_has_no_thump` | A piece that settled plays `Lock` alone. A hard drop adds `HardDrop`, louder for 0, 1, 4 and 10 rows in turn, and no louder for 21. The next lock is an ordinary one again. |
| `a_bigger_piece_lands_heavier` | `Lock` is louder for 4 cells than for 3, and for 5 than for 4. |
| `each_count_of_rows_has_its_own_cue_and_ducks_the_theme_more` | 1 to 5 rows play `Clear1` to `Clear5`. One row leaves the theme alone. From two rows on, each count lowers it further; it comes back without dipping again, to exactly its own level. |
| `a_level_up_plays_its_cue_and_holds_the_theme_down_longer_than_a_clear` | A level-up holds the theme down longer than a four-row clear does, and a clear with a level-up longer still. |
| `a_pause_cuts_what_was_ringing_and_fades_the_theme_out_and_back` | A pause in the middle of a four-row clear stops every cue but the theme, plays `Pause` once, and takes the theme to silence in about `FADE_OUT` from where the clear had lowered it, without ever raising it. A long pause orders nothing more. A resume plays `Resume` once and brings the theme back in about `FADE_IN`, not held down by the clear the pause cut. |
| `the_end_stops_the_theme_and_nothing_brings_it_back` | `GameOver` stops the theme and a level-up cue from the same lock, and plays its own cue. No later frame changes the theme's volume. |
| `a_new_game_silences_the_old_one_and_starts_the_theme_again` | `Desk::restart` first stops every cue, then loops the theme and plays `NewGame`. Nothing of the old game is left: no duck, no ending, no soft-drop loop. |
| `the_title_screen_plays_the_title_theme_and_nothing_else` | `Desk::title` loops `Title` once and orders nothing more for 30 s. |
| `a_game_started_from_the_title_fades_the_title_theme_out_under_the_game_theme` | From the title, `restart` does not stop `Title`, starts `Theme` and `NewGame`, and lowers `Title` steadily until it is stopped within `TITLE_FADE`. A later restart has no title left to fade. |
| `back_to_the_title_stops_the_game_and_starts_the_title_theme` | `back_to_title` stops every cue, loops `Title`, and leaves no duck behind. |
| `every_cue_has_a_file_and_a_level_that_cannot_clip_alone` | Every cue has a file compiled in and a level between 0 and 1, and its index is its place in `Cue::ALL`. |

### Staging keys

These tests are in `src/stage.rs` and run only with the `stage` feature. They
prove that each stage leads where the sound bench assumes it does. See
[Controls and timing](controls-and-timing.md#staging-keys).

| Test | What it proves |
| --- | --- |
| `a_hard_drop_on_staged_rows_clears_exactly_that_many` | After F1 to F5, the piece fits, has 3 to 5 cells, and a hard drop clears exactly 1 to 5 rows, leaves blocks in the well and does not end the game. |
| `the_row_after_f6_raises_the_level` | After F6, one cleared row pushes `LevelUp`. |
| `the_lock_after_the_brim_ends_the_game_whatever_piece_comes_next` | After F7 the piece fits, and its lock ends the game without clearing a row, for 300 random next pieces. |
| `an_empty_well_puts_the_piece_back_at_the_top` | F8 empties the well and returns the piece to its spawn row. |

## What has no test

| Area | Behaviour with no test |
| --- | --- |
| `Game::update` | The function itself: reading keys, applying repeat moves, both keys held, and that it calls `sidestep`, `rotate`, `hard_drop` and `descend` at the right moments. Those four are tested. |
| `main` | The order of a frame, the P toggle, P being ignored when the game is over, and the seeding of the random generator. |
| Rendering | `conf`, `draw_piece`, the `Game::draw` family and the three `Fx::draw_*` functions: what is actually drawn, including which moves get a faint copy of the piece. This can only be judged by running the game. |
| Effects in `main` | That `Fx::tick` is skipped during a pause and that a restart replaces `Fx`. |
| Sound in `main` | That every event reaches `Desk::on`, that `Desk::frame` runs every frame, and that a restart calls `Desk::restart`. |
| `Speakers` | `Speakers::load` and `Speakers::obey`: that an order becomes a sound. This needs a sound card. |
| What is heard | The files, their levels and the mix. No test listens. The sound bench checks them: `mise run sound-check` measures the files and the room in the mix, and `mise run sound-record` records the running game. Neither is part of `mise run test`. See [Sound](sound.md#checking-the-sound). |
| `stage::keys` | That each function key calls its stage. The stages themselves are tested. |
| Fall speed | The formula `(0.6 - 0.05 * level).max(0.08)` and the soft-drop value `0.04`. |
| `Game::level` | Not checked directly. One test checks only that `LevelUp` is pushed when `lines` goes from 9 to 11. |
| `Game::lock` | That `after` gets a new piece, and that the piece that ends the game stays in place. |
| `Game::rotate` | A one-column kick, a two-column kick off the right wall, a refusal at the floor, and the refusal for a piece that would reach two rows above row 0. |
| Piece generation | That a piece is connected, and how often each shape and size appears. |

The tests do not seed the random generator. It starts from its default state.
