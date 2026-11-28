# Polyfall music

Three scores for [sonomorph](https://github.com/chussenot/sonomorph)'s advanced engine.
The game plays them from `assets/*.ogg`: see "In the game" below.

| Score | What | Length |
| --- | --- | --- |
| `theme.score` | The gameplay loop. Seamless (`loop on`). | 29 bars, 1:40 |
| `level_up.score` | One bar when the level rises. | 3.7 s and a 3 s tail |
| `game_over.score` | The ending. | 9.6 s and a 4 s tail |

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
shape, in D Dorian. `level_up` runs the three cells upward into D major, the
only major tonic in the game. `game_over` plays the motif once more, slower and
lower, and ends on an open fifth.

`ARR-SPEC.yaml` is the full arrangement: intent, form, harmony, who plays when,
and the checks it was held to. It follows the spec layout of
[music-composition-skills](https://github.com/jtydhr88/music-composition-skills).

## Changing the music

The `.score` files are generated. Edit `compose.py`, then:

```bash
python3 audio/compose.py
```

It needs only Python 3 and always writes the same files (fixed seed). Commit
`compose.py` and the scores together.

## Hearing it

```bash
cargo install --git https://github.com/chussenot/sonomorph \
  --features render,render-advanced --bin sonomorph-render sonomorph
sonomorph-render audio/theme.score theme.wav
```

This needs a C compiler. The theme renders in about 8 seconds.

## In the game

`mise run audio` renders the three scores at 44100 Hz and encodes them to
`assets/*.ogg` with ffmpeg. Those files are committed and compiled into the
binary, so building the game needs neither sonomorph nor ffmpeg. Run the task
again after changing a score.

The theme loops while a game is in play and is muted during a pause. The
level-up cue plays over it. The game-over cue replaces it until R starts a new
game.

## Licence of the sounds

The orchestral samples in sonomorph's library are from VSCO 2 Community
Edition, CC0. The notes are original to Polyfall.
