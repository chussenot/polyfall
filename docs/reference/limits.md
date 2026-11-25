---
title: Limits and edge cases
description: Places where the game behaves in a way a player or a developer might not expect, each with the function that causes it.
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

# Limits and edge cases

This page lists behaviour that follows from the code but that you might not
expect. Read it before you report a bug, to see whether the behaviour is
already known, and before you decide what to improve.

Each entry describes what the code does. None of them says what was intended.

## The one limit the code names

`src/main.rs` has one comment that starts with `shortcut:`. It is in
`Game::rotate`: a turned piece that would reach two or more rows above the top
of the well does not turn, because only one row down is tried. See
[When a turn is refused](rotation-and-kicks.md#when-a-turn-is-refused), which
also lists the other limits of rotation.

## Input

| Behaviour | Cause |
| --- | --- |
| Key repeat can start without its delay. `held` returns to `0.0` on a new press only, not on a release. If Left and Right are pressed in the same frame, neither counts as a press: the piece does not move and `held` keeps its old value. When one of the two keys is then released, repeat moves start from that old value. | `Game::update` |
| Any held key stops the automatic pause, including keys the game does not use. | `main`, `Game::tick_idle` |
| R does nothing during play. To start again in the middle of a game, the player pauses first. | `main`, `Game::can_restart` |
| The key list in the side panel has no row for R. R is named only in the overlays. | `Game::draw_panel` |
| No key quits the game. The loop in `main` has no exit; the player closes the window. | `main` |

## Timing

| Behaviour | Cause |
| --- | --- |
| Fall speed depends on the frame rate. At each downward step `fall` returns to `0.0` instead of keeping the time left over, and the piece moves at most one row per frame. The real time between two steps is the fall interval rounded up to a whole number of frames. The effect is largest for the short intervals, `0.04` s and `0.08` s. | `Game::update` |
| There is no lock delay beyond the fall interval. With Down held, a piece resting on the stack locks after at most `0.04` s plus one frame. | `Game::update` |
| The display timers `flash` and `glow` run during a pause and after the game is over, because `draw` counts them down. `draw` therefore changes the `Game` value. | `Game::draw` |

## Display

| Behaviour | Cause |
| --- | --- |
| The band over removed rows is drawn at the row indices from before the rows above moved down, so it covers rows that now hold other cells. | `Game::lock`, `Game::draw_overlay` |
| `cleared` is not emptied when `flash` reaches zero. The band is still drawn, at alpha 0, until the next lock. | `Game::draw_overlay` |

## Other

| Behaviour | Cause |
| --- | --- |
| The seed of the random generator is the clock in whole seconds. Two games started within the same second get the same pieces. | `main` |
| `score` and `lines` are `u32` and are added to without an overflow check. | `Game::lock` |

## Agreement with `PRODUCT.md`

`PRODUCT.md` describes the product. Its statements about behaviour match the
code: a well of 12 by 22, pieces of 3 to 5 cells coloured by cell count, two
upcoming pieces, a landing outline, a score of rows squared times 100, a level
every 10 rows, pause on P, an automatic pause after 8 seconds, key repeat for
Left and Right, R only when paused or over, no sound, no saved scores, one
source file, and macroquad 0.4 with audio disabled.
