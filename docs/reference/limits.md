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
  - Desk::on
  - Speakers::obey
  - level
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
| A move or turn on the ground is noticed by comparing the piece before and after the keys of a frame. A move that cancels itself within one frame (Left and Right pressed together) does not restart the lock delay. | `Game::update` |

## Display

| Behaviour | Cause |
| --- | --- |
| For up to about `0.4` s after rows are removed, the stack is drawn higher than it is while the rows come down. Collision and the landing outline use the real well. More limits of the effects are on [Effects](effects.md#limits). | `Fx::block` |
| `Game::events` is emptied only by `main`. Code that drives a `Game` without that loop, as the tests do, lets it grow. | `main` |

## Sound

| Behaviour | Cause |
| --- | --- |
| A sound starts up to 93 ms after the event that caused it, on top of the delay of the sound card. macroquad's mixer starts sounds only at the beginning of a block of 4096 samples. | `Speakers::obey` |
| The levels of the sounds were set from measurements, not by ear. More limits of the sound are on [Sound](sound.md#limits). | `level` |
| A piece that falls by itself makes no sound until it locks. Only rows fallen with Down held are heard. | `Desk::on` |
| A held Left or Right that keeps pushing at a wall is silent. Only the press itself is answered when it cannot move. | `Game::sidestep` |

## Other

| Behaviour | Cause |
| --- | --- |
| `score` and `lines` are `u32` and are added to without an overflow check. | `Game::lock` |

## Agreement with `PRODUCT.md`

`PRODUCT.md` describes the product. Its statements about behaviour match the
code: a well of 12 by 22, pieces of 3 to 5 cells coloured by cell count, two
upcoming pieces, a landing outline, a score of rows squared times 100, a level
every 10 rows, pause on P, an automatic pause after 8 seconds, key repeat for
Left and Right, R only when paused or over, M to mute, a theme and sound
effects, one saved best score, and macroquad 0.4.
