# Polyfall music and sound

Scores for [sonomorph](https://github.com/chussenot/sonomorph)'s advanced engine:
three of music, and fifteen sound effects in `sfx/`. The game plays them from
`assets/`: see "In the game" below.

| Score | What | Length |
| --- | --- | --- |
| `theme.score` | The gameplay loop. Seamless (`loop on`). | 29 bars, 1:40 |
| `level_up.score` | The level rises. Plays over the theme. | 2.7 s and a 1.6 s tail |
| `game_over.score` | The ending. | 9.6 s and a 4 s tail |
| `sfx/*.score` | One sound effect each. See "The sound effects" below. | 0.05 to 2.1 s |

## The idea

A piece of the game has 3, 4 or 5 cells, and the well is 12 columns wide.
3 + 4 + 5 = 12. So every bar of the music has 12 pulses, split 3 + 4 + 5, and
each size has its own voice, in the order of the game's colours:

| Cell | Pulses | Voice |
| --- | --- | --- |
| 3 (blue) | 1 to 3 | celli, plucked |
| 4 (orange) | 4 to 7 | violas, short bow strokes |
| 5 (pale sand) | 8 to 12 | a glass-like synth |

Three ticks mark the first pulse of each cell. That uneven count is the thing
to remember the piece by. The melody is one motif with the same 3 + 4 + 5
shape, in D Dorian. `game_over` plays the motif once more, slower and lower,
and ends on an open fifth.

`level_up` runs the three cells upward once, six times faster than the theme,
into an open chord. It plays over the theme at whatever bar the player is in,
so it uses only D, E and A (no third: it cannot be major against a minor bar)
and has no pulse of its own (twelve notes in half a second are a gesture, not
a rhythm that could fall out of step). It starts a quarter of a second late,
after the row clear that caused it.

The theme's tempo is 207.96 pulses a minute, not 208. At that tempo its 29 bars
last exactly 1081 blocks of 4096 samples, which is what the game's mixer needs
to loop it without a glitch. `compose.py` computes the number.

`ARR-SPEC.yaml` is the full arrangement: intent, form, harmony, who plays when,
and the checks it was held to. It follows the spec layout of
[music-composition-skills](https://github.com/jtydhr88/music-composition-skills).

## The sound effects

`sfx.py` writes one score per effect into `sfx/`. There are two materials, so
the ear can tell what kind of thing happened:

| Material | Effects | Made of |
| --- | --- | --- |
| Contact: what the piece physically does | `move_left`, `move_right`, `rotate`, `blocked`, `soft_drop`, `lock`, `hard_drop` | Short filtered noise and low sines. Dry, no pitch to speak of. |
| Glass: what the rules do | `clear_1` to `clear_5`, `pause`, `resume`, `new_game` | A struck-bar tone: three sines at 1, 2.76 and 5.40 times the pitch. |

Every tuned note is a D, an E or an A, the notes that sit over every chord of
the theme. A row clear plays one glass note per row, so the count can be heard.

Each effect starts on its first sample and peaks at -3 dBFS. The `TRIM` table
at the top of `sfx.py` holds the gain that brings each one there. How loud an
effect plays in the game is not in its file: it is the `level()` table in
`src/sound.rs`. `docs/reference/sound.md` is the cue sheet: which event plays
which effect, and at what level.

`soft_drop` is a loop. It lasts exactly 4 mixer blocks, for the reason the
theme lasts 1081.

## Changing the music or an effect

The `.score` files are generated. Edit `compose.py` (music) or `sfx.py`
(effects), then:

```bash
python3 audio/compose.py
python3 audio/sfx.py
mise run audio
```

The scripts need only Python 3 and always write the same files (fixed seeds).
Commit the script, the scores and the rendered files together.

After changing what an effect is made of, bring it back to -3 dBFS:
`python3 tools/sound/bench.py trim --write` measures every effect and writes
the `TRIM` table; run `sfx.py` and `mise run audio` again after it. Then
`mise run sound-check`. `python3 tools/sound/bench.py layers lock` shows the
level of each layer inside an effect. `tools/sound/README.md` has the rest.

## Hearing it

```bash
cargo install --git https://github.com/chussenot/sonomorph \
  --features render,render-advanced --bin sonomorph-render sonomorph
sonomorph-render audio/theme.score theme.wav
```

This needs a C compiler. The theme renders in about 8 seconds.

## In the game

`mise run audio` renders every score at 44100 Hz. The three music scores are
encoded to `assets/*.ogg` with ffmpeg. The effects go to `assets/sfx/*.wav` as
rendered: the game's Ogg decoder pads the end of a short file, and the
soft-drop loop must keep its exact length. All the files are committed and
compiled into the binary, so building the game needs neither sonomorph nor
ffmpeg. Run the task again after changing a score.

The theme loops while a game is in play and fades out during a pause. It is
lowered for a moment under a row clear of two rows or more and under the
level-up cue. The game-over cue replaces it until R starts a new game.
`src/sound.rs` holds these rules.

## Licence of the sounds

The orchestral samples in sonomorph's library are from VSCO 2 Community
Edition, CC0. The effects use no samples: they are synthesised. The notes and
the sounds are original to Polyfall.
