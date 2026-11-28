---
title: Development
description: How to run the game, change it, test it and build this documentation with the mise tasks, and the build facts that are not obvious.
source:
  - mise.toml
  - Cargo.toml
  - mkdocs.yml
  - docs/js/mermaid-init.js
  - src/main.rs
  - src/fx.rs
  - src/sound.rs
  - src/stage.rs
  - examples/decode.rs
  - tools/sound/bench.py
  - tools/sound/record.sh
---

# Development

This page is for a developer who has just cloned the repository. It lists the
commands to run, test and document the game, then the facts that the commands
do not tell you.

## Set up

The project uses [mise](https://mise.jdx.dev) to install its tools and to name
its commands. `mise.toml` declares four tools:

| Tool | Version | Used for |
| --- | --- | --- |
| `rust` | `latest` | Compiler, `cargo`, `rustfmt` and `clippy`. |
| `mr-boxington` | `latest` | Shared build cache. See [Shared build cache](#shared-build-cache). |
| `watchexec` | `latest` | Restarts the game when a file changes (`dev` task). |
| `pipx:mkdocs` | `1.6.1` | Builds this documentation. |

From the repository root, run once:

```bash
mise trust     # allow mise to read mise.toml
mise install   # install the four tools
```

## Tasks

Run a task with `mise run <task>`.

| Task | Command it runs | What it does |
| --- | --- | --- |
| `run` | `cargo run --release` | Builds the optimised binary and starts the game. |
| `dev` | `watchexec --restart --exts rs,toml -- cargo run` | Starts a debug build of the game and restarts it each time a `.rs` or `.toml` file changes. |
| `test` | `cargo test --all-features` | Runs the unit tests in `src/main.rs`, `src/fx.rs`, `src/sound.rs` and `src/stage.rs`. No window opens and no sound plays. |
| `fmt` | `cargo fmt` | Formats the source. |
| `lint` | `cargo fmt --check`, then `cargo clippy --all-targets --all-features -- -D warnings` | Fails if the source is not formatted or if clippy reports any warning. |
| `build` | `cargo build --release` | Builds the optimised binary at `target/release/polyfall` without starting it. |
| `docs` | `mkdocs serve` | Serves this documentation locally and reloads the page when a file changes. |
| `docs-build` | `mkdocs build --strict` | Builds the documentation into `site/` and fails on any warning, such as a broken link. |
| `check` | `lint`, `test` and `docs-build` | Everything that must pass before a commit. |
| `audio` | `sonomorph-render` for each score, then `ffmpeg` for the music | Renders the scores in `audio/` to `assets/*.ogg` and those in `audio/sfx/` to `assets/sfx/*.wav`. Needs `sonomorph-render` and ffmpeg. |
| `sound-check` | `python3 tools/sound/bench.py check` | Checks the sound files against what the mixer needs and searches the mix for a sum over full scale. Needs `python3` with `numpy`. See [The sound bench](#the-sound-bench). |
| `sound-record` | `tools/sound/record.sh --all` | Runs the game on a virtual sound card, presses its keys and checks what it played. Linux only; takes three minutes. |
| `screenshot` | `import -window Polyfall docs/assets/screenshot.png` | Captures the running game window to `docs/assets/screenshot.png`. Needs ImageMagick and X11 or XWayland, and the game must be running. |

A usual working session:

1. `mise run dev` in one terminal, and edit a file in `src/`.
2. `mise run test` after a change to game logic.
3. `mise run sound-check` after a change to a sound or a level.
4. `mise run check` before a commit, and `mise run sound-record` as well if
   the commit touches `src/sound.rs`.

## Facts to know

### Audio

`Cargo.toml` declares one dependency:

```toml
macroquad = { version = "0.4", features = ["audio"] }
```

The `audio` feature needs the ALSA development headers to build on Linux.

The game plays 18 sounds, compiled into the binary with `include_bytes!`:

| Files | What | Rendered from |
| --- | --- | --- |
| `assets/*.ogg`, 3 files | The theme, the level-up cue and the game-over cue. | `audio/*.score`, written by `audio/compose.py` |
| `assets/sfx/*.wav`, 15 files | The sound effects. | `audio/sfx/*.score`, written by `audio/sfx.py` |

`mise run audio` renders them all; it needs `sonomorph-render` and ffmpeg, and
`audio/README.md` says how to install the first. The rendered files are
committed, so building the game needs neither tool. After changing a score,
run the task again and commit the files with it.

Three facts about macroquad's mixer decide how the files are made.
[Sound](reference/sound.md#the-mixer) gives the evidence.

- It mixes at 44100 Hz and resamples anything else by nearest neighbour, so the
  files are rendered at that rate.
- It loops a sound cleanly only if the sound lasts a whole number of blocks of
  4096 samples. The theme and the soft-drop loop are made to that length.
- Its Ogg decoder pads the end of a short file. The effects are therefore WAV,
  which it reads exactly.

### The sound bench

`tools/sound` holds the tools for working on the sound. Its manual is
`tools/sound/README.md`. In short:

| Part | What it does |
| --- | --- |
| `tools/sound/bench.py` | Measures the files and the mix: a cue sheet, an estimate of what the theme covers, spectrograms, and a search of every way sounds can pile up for a sum over full scale. |
| `tools/sound/record.sh` | Runs the game in a window on a virtual screen with a virtual sound card, presses the keys of a session and records what the card was sent. The recording is then taken apart into the cues the game played, when and how loud. |
| `tools/sound/sessions/` | Six sessions: every key, the five row clears, a soft drop, a pause, the end of a game, and the theme's loop point. |

Two pieces of the repository exist for it:

- `examples/decode.rs` decodes a sound file exactly as the game's mixer does,
  with the same crate. The bench measures what it writes. This is why
  `Cargo.toml` names `audrey` as a development dependency; macroquad already
  brings it in, so nothing more is built.
- `src/stage.rs` is compiled only with `--features stage`. It adds eight
  function keys that set a situation up, such as four rows about to clear. See
  [Controls and timing](reference/controls-and-timing.md#staging-keys).
  `record.sh` builds that variant into `target/stage/`, so
  `target/release/polyfall` is always the game as shipped.

Everything the bench writes goes to `target/sound/`.

### Tests run without a window

`cargo test` opens no window, so macroquad has no frame, no keyboard and no
drawing surface during a test. Code under test must not call a macroquad
drawing, time or input function such as `draw_rectangle`, `get_frame_time` or
`is_key_pressed`.

The source keeps to this rule in one way: logic that depends on time or keys
receives them as arguments. `Game::tick_idle(dt, key_active)` and
`Game::repeats(before, after)` are tested; their caller reads the frame time and
the keyboard and passes the values in. Follow the same pattern for new logic.

Only these functions call drawing, time or input functions, and none of them is
tested: `main`, `Game::update`, `Game::draw`, `Game::draw_well`,
`Game::draw_panel`, `Game::draw_overlay`, `draw_piece`, `Fx::draw_under`,
`Fx::draw_clear` and `Fx::draw_over`. `Game::update` keeps what it can in
functions that read no key: `Game::sidestep`, `Game::descend`, `Game::rotate`
and `Game::hard_drop` are tested.

Effects follow the same rule. `Fx::on(event)` and `Fx::tick(dt)` receive what
happened and how much time passed, and the functions that say how to draw
(`Fx::block`, `Fx::piece_pose`, `Fx::score` and the others) only compute. All
of them are tested without a window.

Sound follows it too. `Desk::on(event)` and `Desk::frame(paused, dt)` only
write `Order` values, and are tested without a sound card. `Speakers::load` and
`Speakers::obey` are the only functions that call `macroquad::audio`, and they
are not tested.

macroquad's random generator (`rand::gen_range`) works without a window.
`random_piece` uses it and is tested.

[Tests](reference/tests.md) lists what each test proves.

### Shared build cache

`mise.toml` installs `mr-boxington` and turns it on for the Rust toolchain with
the option `mr_boxington = true` on the `rust` tool. It is a build cache that
reuses compiled work across every checkout and git worktree on the machine. It changes how fast a build is, not what is
built, and the game code does not refer to it.

### How the documentation is built

- `mkdocs.yml` lists the pages under `nav`. A new page must be added there.
- No mkdocs plugin or theme is installed. Diagrams are Mermaid code blocks.
  `mkdocs.yml` loads Mermaid 10.9.1 from a CDN, and `docs/js/mermaid-init.js`
  turns each `mermaid` code block into a diagram in the browser. Diagrams
  therefore need network access to appear.
- `mkdocs build` writes to `site/`, which is ignored by git.
- Every page starts with front matter: `title`, `description` and `source`.
  `source` names the functions, constants and files the page describes. Update
  the page when one of them changes.

### Three source files

The game and its tests are in `src/main.rs`. The visual effects and their tests
are in the module `src/fx.rs`, the sound and its tests in the module
`src/sound.rs`. The only assets are the sounds in `assets/`, and there are no
configuration files. [Components](architecture/components.md) maps the three
files. The small module `src/best.rs` reads and writes the best score.

A fourth file, `src/stage.rs`, is not part of the game as shipped. See
[The sound bench](#the-sound-bench).
