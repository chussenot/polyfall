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
| `test` | `cargo test` | Runs the unit tests in `src/main.rs` and `src/fx.rs`. No window opens. |
| `fmt` | `cargo fmt` | Formats the source. |
| `lint` | `cargo fmt --check`, then `cargo clippy --all-targets -- -D warnings` | Fails if the source is not formatted or if clippy reports any warning. |
| `build` | `cargo build --release` | Builds the optimised binary at `target/release/polyfall` without starting it. |
| `docs` | `mkdocs serve` | Serves this documentation locally and reloads the page when a file changes. |
| `docs-build` | `mkdocs build --strict` | Builds the documentation into `site/` and fails on any warning, such as a broken link. |
| `check` | `lint`, `test` and `docs-build` | Everything that must pass before a commit. |
| `audio` | `sonomorph-render`, then `ffmpeg`, for each score | Renders the scores in `audio/` to `assets/*.ogg`. Needs `sonomorph-render` and ffmpeg. |
| `screenshot` | `import -window Polyfall docs/assets/screenshot.png` | Captures the running game window to `docs/assets/screenshot.png`. Needs ImageMagick and X11 or XWayland, and the game must be running. |

A usual working session:

1. `mise run dev` in one terminal, and edit `src/main.rs` or `src/fx.rs`.
2. `mise run test` after a change to game logic.
3. `mise run check` before a commit.

## Facts to know

### Audio

`Cargo.toml` declares one dependency:

```toml
macroquad = { version = "0.4", features = ["audio"] }
```

The `audio` feature needs the ALSA development headers to build on Linux.

The game plays three sounds from `assets/*.ogg`, compiled into the binary with
`include_bytes!`. `mise run audio` renders them from the scores in `audio/`; it
needs `sonomorph-render` and ffmpeg, and `audio/README.md` says how to install
the first. The files are rendered at 44100 Hz because macroquad mixes at that
rate and resamples anything else by nearest neighbour.

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
`Fx::draw_clear` and `Fx::draw_over`.

Effects follow the same rule. `Fx::on(event)` and `Fx::tick(dt)` receive what
happened and how much time passed, and the functions that say how to draw
(`Fx::block`, `Fx::piece_pose`, `Fx::score` and the others) only compute. All
of them are tested without a window.

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

### Two source files

The game and its tests are in `src/main.rs`. The visual effects and their tests
are in `src/fx.rs`, the only module. The only assets are the three sounds in
`assets/`, and there are no configuration files. [Components](architecture/components.md) maps both files.
