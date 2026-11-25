---
title: Limits and inconsistencies
description: Limits the code documents itself, edge cases found while reading it, and places where the code disagrees with PRODUCT.md.
source:
  - Game::rotate
  - Game::update
  - Game::tick_idle
  - Game::lock
  - Game::draw
  - Game::draw_panel
  - Game::draw_overlay
  - main
  - PRODUCT.md
---

# Limits and inconsistencies

This page collects observations made while reading the code. None of them is a
statement about intent. Each one names the code it comes from.

## Limit documented in the code

`src/main.rs` contains one `shortcut:` comment, in `Game::rotate`:

```text
shortcut: a single row down, so a shape poking 2+ rows above row 0 still refuses; add more rows if that matters
```

Only one downward kick row is tried, so a rotation that would put cells two or
more rows above the top of the well is refused. See
[Rotation and kicks](rotation-and-kicks.md).

## Edge cases

| Area | Observation | Code |
| --- | --- | --- |
| Key repeat | `held` is reset by a new press only. If Left and Right are pressed in the same frame and one is later released, or if a key is held from a pause into play, repeat moves start from whatever `held` contained, without a first move and possibly without the delay. | `Game::update` |
| Gravity | `fall` is set to `0.0` at each step instead of being reduced by the interval, and at most one row is descended per frame. Actual fall speed depends on the frame rate; the difference is largest for the short intervals (`0.04`, `0.08`). | `Game::update` |
| Lock | There is no lock delay other than the current fall interval. With Down held, a resting piece locks after at most `0.04` s plus one frame. | `Game::update` |
| Idle pause | Any held key resets the idle count, including keys the game does not read. | `main`, `Game::tick_idle` |
| Feedback timers | `flash` and `glow` are decreased in `draw`, so they run during a pause and after game over. `draw` therefore changes game state. | `Game::draw` |
| Cleared-row band | `cleared` is not emptied when `flash` reaches zero. The band rectangles are still drawn, with alpha 0, until the next lock replaces the list. | `Game::draw_overlay` |
| Cleared-row band | The band is drawn at the row indices from before the collapse, over rows that have since moved down. | `Game::lock`, `Game::draw_overlay` |
| Random seed | The seed is the clock in whole seconds. Two starts within the same second produce the same sequence of pieces. | `main` |
| Rotation | One direction only, around the first generated cell rather than the centre of the shape. No upward kick. | `Game::rotate` |
| Restart | R works only when the game is over or paused. To restart during play, the player must pause first. | `main`, `Game::can_restart` |
| Key list | The on-screen key list has no row for R. R is shown only in the two overlays. | `Game::draw_panel` |
| Exit | The loop in `main` has no exit condition and no key quits the game. | `main` |
| Counters | `score` and `lines` are `u32` and are added to without overflow handling. | `Game::lock` |

## Differences between the code and `PRODUCT.md`

| `PRODUCT.md` says | The code does |
| --- | --- |
| "No sound, pause, saved high scores or key auto-repeat today." | Pause exists (P, and idle auto-pause after `IDLE_PAUSE`). Key repeat exists for Left and Right (`REPEAT_DELAY`, `REPEAT_RATE`). Sound and saved scores are absent, as stated. |
| "Keyboard only: arrows to move, rotate and soft-drop, Space to hard-drop, R to restart." | Also P to pause. R restarts only when the game is over or paused. |
| "next-piece preview" | Two upcoming pieces are shown: `next` and `after` (labels `NEXT` and `THEN`). |
| "The running game and its six unit tests in `src/main.rs`." | The `tests` module has 13 tests. |

The other statements checked against the code agree with it: a well of 12 by
22, pieces of 3 to 5 cells, score of rows squared times 100, level every 10
rows, a single source file, macroquad 0.4 without default features.
