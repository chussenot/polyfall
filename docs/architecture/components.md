---
title: Components
description: The seven groups of functions in src/main.rs and src/fx.rs, which function calls which, and the order of work in one frame (C4 level 3).
source:
  - main
  - Game
  - Piece
  - random_piece
  - Game::new
  - Game::fits
  - Game::shift
  - Game::landing_y
  - Game::rotate
  - Game::lock
  - Game::level
  - Game::can_restart
  - Game::repeats
  - Game::tick_idle
  - Game::update
  - Game::draw
  - Game::draw_well
  - Game::draw_panel
  - Game::draw_overlay
  - draw_piece
  - Game::hard_drop
  - Event
  - Fx
---

# Components (C4 level 3)

This page is the map of `src/main.rs` and `src/fx.rs`. Read it with the source open, before you
change anything: it tells you which functions belong together and in what order
they run.

## The idea

The whole game is one value of type `Game`, owned by `main`. `main` runs
one frame after another. In each frame it reads the keyboard, changes the `Game`
value, then draws it. Every function in the file either changes that value or
reads it.

The effects are the one exception. They live in a second value of type `Fx`,
also owned by `main`, in the module `src/fx.rs`. The game never reads it.

Apart from that module, the components below are groups of functions that share
a concern. They are a way to read `src/main.rs`, not something the compiler
knows about.

The diagram shows the direction of data: input changes the game state through
two components, and rendering only reads it. Effects hear about changes as
events and are read by rendering; nothing flows back from them.

```mermaid
flowchart LR
    Input["Input and timing"]
    Move["Movement and rotation"]
    Locking["Locking and scoring"]
    Gen["Piece generation"]
    State[("Game state")]
    Render["Rendering"]
    Input --> Move
    Input --> Locking
    Locking --> Gen
    Move --> State
    Locking --> State
    State --> Render
    Fx["Effects"]
    Move -.->|"events"| Fx
    Locking -.->|"events"| Fx
    Fx --> Render
```

## The components

| Component | Functions and types | What it does | Details |
| --- | --- | --- | --- |
| Game state | `Game`, `Piece`, `Grid`, `Event`, `Game::new` | Holds the well, the falling piece, the two upcoming pieces, the counters, the timers, the flags and the events not yet handed to the effects. | [Game state](../reference/game-state.md) |
| Piece generation | `random_piece` | Grows a random connected shape of 3 to 5 cells and places it at the top of the well. | [Game rules](../reference/game-rules.md#how-a-piece-is-made) |
| Movement and rotation | `Game::fits`, `Game::shift`, `Game::landing_y`, `Game::rotate` | Tests whether a piece fits, moves it, finds its landing row and turns it. | [Rotation and kicks](../reference/rotation-and-kicks.md) |
| Locking and scoring | `Game::hard_drop`, `Game::lock`, `Game::level` | Writes the piece into the well, removes full rows, updates the score, brings in the next piece and detects the end of the game. | [Game rules](../reference/game-rules.md#locking-and-clearing-rows) |
| Input and timing | `main`, `Game::update`, `Game::repeats`, `Game::tick_idle`, `Game::can_restart` | Reads keys and applies key repeat, falling, pause, restart and the automatic pause. | [Controls and timing](../reference/controls-and-timing.md) |
| Rendering | `conf`, `Game::draw`, `Game::draw_well`, `Game::draw_panel`, `Game::draw_overlay`, `draw_piece` | Sets up the window and draws the well, the side panel and the overlays. | [Rendering and layout](../reference/rendering-and-layout.md) |
| Effects | `Fx` and everything else in `src/fx.rs` | Turns events into short-lived motion, light and fragments, and tells rendering how to draw them. | [Effects](../reference/effects.md) |

`Game::fits` is the function everything else rests on. It answers one question:
are all these cells inside the well and on empty cells? A piece is moved or
turned only after `fits` says yes.

## Who calls what

| Caller | Calls |
| --- | --- |
| `main` | `Game::new`, `Game::can_restart`, `Game::tick_idle`, `Game::update`, `Fx::on`, `Fx::tick`, `Game::draw` |
| `Game::new` | `random_piece`, three times |
| `Game::update` | `Game::shift`, `Game::repeats`, `Game::rotate`, `Game::hard_drop`, `Game::lock`, `Game::level` |
| `Game::hard_drop` | `Game::landing_y`, `Game::lock` |
| `Game::shift`, `Game::landing_y`, `Game::rotate` | `Game::fits` |
| `Game::lock` | `Game::level`, `random_piece`, `Game::fits` |
| `Game::draw` | `Game::draw_well`, `Game::draw_panel`, `Game::draw_overlay` |
| `Game::draw_well` | `Game::landing_y`, `draw_piece`, `Fx::draw_under`, `Fx::block`, `Fx::draw_clear`, `Fx::piece_pose`, `Fx::draw_over` |
| `Game::draw_panel` | `Game::level`, `Fx::score`, `Fx::score_tint`, `Fx::gain`, `Fx::levelled` |
| `Game::draw_overlay` | `Fx::over_dim`, `Fx::over_box` |

Only `main`, `Game::update`, `draw_piece`, the `Game::draw` family and the
three `Fx::draw_*` functions call macroquad input, time or drawing functions. The rest is plain computation on
`Game`, which is why the tests run without a window. See
[Development](../development.md#tests-run-without-a-window).

## One frame

`main` seeds the random generator, creates a `Game`, then repeats the same
steps forever. The code has no exit path: the process ends when the player
closes the window.

The diagram shows the fixed order of one frame. `update` is the only step that
can be skipped.

```mermaid
flowchart LR
    R["R: new game"]
    P["P: toggle pause"]
    Idle["tick_idle"]
    Upd["update"]
    Fx["effects"]
    Draw["draw"]
    Next["next_frame"]
    R --> P --> Idle --> Upd --> Fx --> Draw --> Next
    Next -->|"next frame"| R
```

| Step | Runs when |
| --- | --- |
| R: replace the game with `Game::new()` and the effects with `Fx::default()` | R was pressed and `can_restart()` is true. |
| P: flip `paused` | P was pressed and the game is not over. |
| `tick_idle` | Every frame. |
| `update` | The game is neither over nor paused. |
| Effects: pass every event in `Game::events` to `Fx::on` | Every frame. |
| Effects: `Fx::tick` | The game is not paused. |
| `draw` | Every frame. |
| `next_frame().await` | Every frame. It hands control to macroquad until the next frame. |

What follows from this order:

- `tick_idle` runs before `update`. When it pauses the game, `update` is
  skipped in that same frame.
- `draw` runs in every state and changes nothing: it takes `&self` and `&Fx`.
- Effects stop during a pause, because `Fx::tick` is skipped, and keep running
  when the game is over, so the ending can play.
- R is tested before P. If both are pressed in one frame while the game is
  paused, a new game starts and P pauses it at once.

## Inside `update`

`Game::update` does the work of a frame of play. Its steps, in order:

1. **Sideways move.** One column on a new press of Left or Right, or the repeat
   moves owed by a held key.
2. **Rotation**, if Up was pressed this frame.
3. **Hard drop**, if Space was pressed this frame. `hard_drop` puts the piece
   on its landing row and calls `lock`, and `update` returns. Step 4 is skipped.
4. **Falling.** `fall` grows by the frame time. When it reaches the current
   fall interval, it returns to `0.0` and the piece moves down one row. If the
   piece cannot move down, `lock` is called.

Steps 1 and 2 come before step 3, so a move or a rotation pressed in the same
frame as Space is applied before the drop.
