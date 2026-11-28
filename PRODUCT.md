# Product

<!-- impeccable:product-schema 1 -->

## Platform

desktop

Native desktop window (Rust + macroquad), keyboard only. Not a web, iOS or Android
product; browser and touch builds are out of scope.

## Users

One player: the author. Played in short breaks between tasks, at the same desk and
keyboard used for work. No other audience is planned, so nothing needs explaining to
a first-time stranger.

## Product Purpose

Polyfall is a small falling-block puzzle game. Pieces fall into a well, full rows
clear, and the game speeds up as rows are cleared. Success is a session that starts
instantly, is fun for a few minutes, and can be dropped without loss.

## Positioning

Every piece is a randomly grown connected shape of 3 to 5 cells, so there is no fixed
piece set to memorise. The game is about reading an unfamiliar shape quickly and
adapting to it. Random pieces are the identity of the game, not a side feature.

## Operating Context

- Launched from a terminal with `mise run run` (or `cargo run --release`), inside a development session.
- Sessions are short and interruptible; the player may leave at any moment.
- Keyboard only: arrows to move, rotate and soft-drop, Space to hard-drop, P to pause,
  R for a new game (only when paused or over).

## Capabilities and Constraints

- Well of 12 by 22 cells; pieces of 3 to 5 cells, coloured by cell count; the two
  upcoming pieces are shown; a landing outline shows where the piece will rest.
- Score is rows cleared at once, squared, times 100; level rises every 10 rows.
- Pause on P, automatic pause after 8 seconds without a key press, key repeat for
  held Left/Right.
- A looping theme, a level-up cue and a game-over cue, and a sound for every action of the
  player: move, rotate, a blocked move, soft drop, lock, hard drop, row clears of 1 to 5 rows,
  pause, resume and new game. All composed in `audio/`. No saved high scores today.
- Three source files: the game in `src/main.rs`, its visual effects in `src/fx.rs`, its sound in `src/sound.rs`. macroquad 0.4 with its audio feature, which
  needs the ALSA headers to build on Linux. A fourth file, `src/stage.rs`, is compiled only for the
  sound bench in `tools/sound` and is not in the game as shipped.
- It is an original game in the falling-block genre. It must not copy the name,
  piece set, look or music of any existing game.

## Brand Commitments

- Name: Polyfall.

## Evidence on Hand

- The running game and its unit tests in `src/`; reverse documentation in `docs/`.
- One screenshot at `docs/assets/screenshot.png`.
- Three music scores in `audio/`, with their arrangement spec, rendered to `assets/*.ogg` and played by the game.
- Fifteen sound-effect scores in `audio/sfx/`, rendered to `assets/sfx/*.wav` and played by the game.
  Their levels were set from measurements; nobody has tuned them by ear yet.
- No logo, art or player feedback exist; none should be invented.

## Product Principles

1. Random shapes come first: any change should make unfamiliar pieces easier to read
   and more interesting to place, never more predictable.
2. Instant in, instant out: no menus or setup between launching and playing.
3. Built for one player at a desk: keyboard only, no onboarding, no accounts.
4. Stay small: a feature must earn its place in a one-file game.
