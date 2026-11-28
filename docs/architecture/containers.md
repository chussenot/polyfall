---
title: Containers
description: The single polyfall binary, what it uses the macroquad library for, and the parts a program often has that this one lacks (C4 level 2).
source:
  - Cargo.toml
  - Cargo.lock
  - main
  - src/fx.rs
  - src/sound.rs
  - conf
---

# Containers (C4 level 2)

This page shows what the program is made of when it runs. Read it to learn
which work the game code does itself and which it leaves to macroquad.

There is one unit to build and run: the `polyfall` binary (package `polyfall`,
version `0.1.0`, Rust edition 2021). Cargo links two things into it: the game
code from `src/main.rs`, `src/fx.rs` and `src/sound.rs`, and the macroquad
library. The sounds in `assets/` are compiled into the binary too.

The diagram shows that the game code never talks to the operating system
directly: every call goes through macroquad.

```mermaid
flowchart LR
    subgraph Binary["polyfall binary"]
        Code["Game code: src/main.rs, src/fx.rs, src/sound.rs"]
        MQ["macroquad 0.4"]
    end
    OS["Window, keyboard, clock, sound card"]
    Code -->|"calls"| MQ
    MQ -->|"calls"| OS
```

## The game code

Three files, and a fourth, `src/stage.rs`, that is compiled only for the
[sound bench](../development.md#the-sound-bench). `src/fx.rs` holds the visual effects: the `Fx` type and its tests.
See [Effects](../reference/effects.md). `src/sound.rs` holds the sound: the
`Desk` and `Speakers` types and their tests. See
[Sound](../reference/sound.md). `src/main.rs` holds the rest: the
constants, the `Piece`, `Game` and `Event` types,
the free functions `random_piece`, `draw_piece`, `conf` and `main`, and the
`tests` module. [Components](components.md) describes its parts.

## macroquad

macroquad is the only direct dependency. The game imports it with
`use macroquad::prelude::*`, and `src/sound.rs` imports `macroquad::audio`. The
library does six jobs:

| Job | Calls in the code |
| --- | --- |
| Program entry and frame loop | `#[macroquad::main(conf)]`, `Conf`, `next_frame().await` |
| Keyboard | `is_key_pressed`, `is_key_down`, `get_keys_down`, `KeyCode` |
| Time | `get_frame_time`, `macroquad::miniquad::date::now` |
| Random numbers | `rand::srand`, `rand::gen_range` |
| Drawing | `set_camera`, `Camera2D::from_display_rect`, `Rect`, `Color`, `clear_background`, `draw_rectangle`, `draw_rectangle_ex`, `draw_rectangle_lines`, `draw_text`, `measure_text` |
| Sound | `load_sound_from_bytes`, `play_sound`, `stop_sound`, `set_sound_volume`, `PlaySoundParams`, `Sound`, from `macroquad::audio`. Only `Speakers` calls them. |

macroquad is built with its `audio` feature. [Development](../development.md#audio)
says what that needs and where the sounds come from.

The game loads no font. `draw_text` and `measure_text` use the font built into
macroquad.

## What is not there

| Absent | Evidence in the source |
| --- | --- |
| Files | No `std::fs` call and no path. The 18 sounds in `assets/` are compiled into the binary with `include_bytes!`. |
| Network | No socket or HTTP call, and no networking crate in `Cargo.lock`. |
| Configuration | No command-line argument, environment variable or settings file is read. Every tuning value is a `const` or a literal in the code. |
| Threads and background work | `main` is `async` because macroquad requires it. It awaits `Speakers::load()` once, at start, and `next_frame()` every frame. macroquad runs its mixer on a thread of its own. |
| Other input devices | No mouse, touch or gamepad call. |
