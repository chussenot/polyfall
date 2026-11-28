# The sound bench

Tools for working on Polyfall's sound: measuring the files and the mix, and
recording the running game to see what it really played.

It answers the questions that come up every time a sound or a level changes:

- Is the file right for the game's mixer?
- How loud is this cue next to the theme, and next to the others?
- Can the mix pass full scale?
- Did the game play the cue I expect, when I expect it, at the volume I expect?

What each sound is meant to be is in
[`docs/reference/sound.md`](../../docs/reference/sound.md). How the files are
made is in [`audio/README.md`](../../audio/README.md).

## In short

```bash
mise run sound-check       # the files and the room in the mix; 5 seconds, no sound card
mise run sound-record      # run the game, press its keys, check what it played; 3 minutes
```

Both end with `all good` or with the problems, one per line, and a non-zero
exit status. Everything they write goes to `target/sound/`, which is not in
git.

## What you need

| For | You need |
| --- | --- |
| Everything | `python3` with `numpy`, and what builds the game. |
| `record.sh` | Linux, bash 5, `Xvfb`, `xdotool`, PulseAudio with its command-line tools, and ALSA's PulseAudio plugin. On Debian or Ubuntu: `apt-get install xvfb xdotool pulseaudio pulseaudio-utils libasound2-plugins`. |
| `bench.py layers` and `trim` | `sonomorph-render` on the `PATH`, or its path in `SONOMORPH_RENDER`. |
| `bench.py listen` | `ffmpeg` for MP3. Without it the files are WAV. |

## The working loop

1. Change the sound. A level is a number in `level()` in `src/sound.rs`. A
   timbre is a function in `audio/sfx.py` or `audio/compose.py`, followed by
   `mise run audio`.
2. `python3 tools/sound/bench.py trim --write` if a timbre changed. It brings
   every effect back to -3 dBFS. Then `python3 audio/sfx.py && mise run audio`
   again.
3. `mise run sound-check`.
4. `python3 tools/sound/bench.py sheet` and `heard` for the balance.
5. `python3 tools/sound/bench.py listen`, and listen.
6. `mise run sound-record` before committing.

## Measuring: `bench.py`

```bash
python3 tools/sound/bench.py <command>
```

| Command | What it tells you |
| --- | --- |
| `check` | The files and the room in the mix, pass or fail. See [What `check` checks](#what-check-checks). |
| `sheet` | Every cue as the game plays it: peak, loudness, and loudness against the theme. |
| `heard` | How far each effect stands above the level at which the theme covers it. An estimate. |
| `spec [cue...]` | A spectrogram of each cue, as a PNG in `target/sound/spec/`. |
| `layers <cue>...` | The level of each layer inside an effect, each rendered alone. |
| `trim [--write]` | The `TRIM` table that brings each effect to -3 dBFS. `--write` puts it into `audio/sfx.py`. |
| `take <session>` | What the game played in a recording, and whether that was right. `record.sh` runs it. |
| `seam <session>` | What a recording holds at the theme's loop point. |
| `listen` | Files to listen to, in `target/sound/listen/`: every sound alone, the mix over the theme's loudest bars, each take. |

Cues go by the name of their file: `move_left`, `clear_4`, `theme`.

The bench keeps no copy of the mix. It reads the levels, the file names and
the timings out of `src/sound.rs`, and it decodes the files with the decoder
the game uses (`examples/decode.rs`). When the source changes shape it stops
and says what it could not find.

### What `check` checks

The files, as the game decodes them:

| Check | Why |
| --- | --- |
| 44100 Hz | The mixer runs at that rate. It resamples anything else by nearest neighbour. |
| A looped cue lasts a whole number of 4096-frame blocks. | The mixer plays the loop point wrong otherwise. See [The mixer](../../docs/reference/sound.md#the-mixer). |
| Every effect and the level-up peak at -3 dBFS. | The levels in `src/sound.rs` assume it. |
| An effect starts within 5 ms of its file's start. | The mixer already adds up to 93 ms. |

The room in the mix. The mixer adds every sound up and has no limiter, so a
sum above full scale is clipped by the sound card. Adding peaks up does not
tell whether that can happen: two attacks can cancel as well as add, and the
theme is only loud in places. But the mixer starts every sound on a block of
4096 frames, and the theme is 1081 blocks long. So a group of cues can meet
the theme in 1081 ways and no more, and `check` tries them all:

| Case | Placements tried |
| --- | --- |
| Everything that can start on one block: a lock with its hard drop at full strength, 0 to 5 rows cleared, a level-up, and beside them any of a move, a turn and a refused key. | 132 groups, each at 1081 places in the theme |
| Two locks, the second 1 to 11 blocks after the first, while the first still rings. | 1056 pairs, each at 1081 places |
| Busy play, made up at random, 12 times through the theme. | Not exhaustive: a sample of what three or more overlapping events do. |

Any placement over full scale fails the check. The line "if its highest sample
met the theme's" is the bound for a mixer whose blocks vary in length, which
is the case on Windows: there a group can meet the theme anywhere.

The search takes a cue's level for the loudest the cue gets. Every take is
checked for that: a cue recorded above its level fails the take.

## Recording: `record.sh`

```bash
tools/sound/record.sh --list       # the sessions there are
tools/sound/record.sh clears       # record one and judge it
tools/sound/record.sh --all        # all of them, then a summary
```

A session is a list of key presses. `record.sh` builds the game, runs it in a
window on a virtual screen with a virtual sound card, sends the keys through
the X server, and records what the sound card was sent. The screen and the
card exist for the length of the take and belong to it alone: nothing is heard
or seen on the machine, and its own sound server is not touched.

The recording is then taken apart. Since the mixer starts every sound on a
block, each block of the recording is the theme times a volume, plus the cues
that start there times their own. Fitting that gives the list of what the game
played, when and how loud, the theme's volume over time, and what is left
over. For a healthy game nothing is left over.

```
   3.288  KEY   space
   3.509        theme volume 0.24
   3.509    cue clear_4    volume 0.497
   3.509    cue hard_drop  volume 0.561
   3.509    cue level_up   volume 0.284
   3.509    cue lock       volume 0.346
   3.744  KEY   p
   3.973        theme volume 0.16
   3.973    cue pause      volume 0.142
   4.159        theme volume 0.00

recording: peak -6.07 dBFS, 0 samples over full scale
not explained by the theme and the cues: -152.2 dBFS overall, -133.4 dBFS in the worst block
cues: as the session expects (clear_4+hard_drop+level_up+lock | pause | resume)
take: all good
```

A volume is what the game asked the mixer for: the cue's balance, times the
master, times what the event adds (a lighter piece, a shorter fall).

A take fails when:

- the cues played are not the ones the session expects;
- a cue played above its level;
- a sample passes full scale (takes are recorded in floats, so such a sample
  is kept as it was);
- a block holds more than -60 dBFS that the theme and the cues do not explain.

A take is in `target/sound/takes/<session>/`: `out.wav`, the `keys` sent and
when, the `session.sh` that sent them, and `game.log`.

### The sessions

| Session | What it plays | Needs |
| --- | --- | --- |
| `tour` | Every key a player has, on random pieces. No cue list to expect: read the timeline. | The game as shipped |
| `clears` | One to five rows cleared by a hard drop. The four-row clear raises the level. | Staging keys |
| `soft` | A soft drop to the floor and its lock. | Staging keys |
| `pause` | A pause in the middle of the biggest sound, then a resume. | Staging keys |
| `over` | The lock that ends the game, the ending, a new game during it. | Staging keys |
| `mute` | A row clear while muted, which must not be heard, then one after unmuting. | Staging keys |
| `loop` | The theme alone, once round and past its loop point. Then `seam`. | Staging keys |

### Staging keys

Random pieces take minutes to produce five full rows. Built with
`--features stage`, the game listens to eight more keys that set a situation
up. What happens next is ordinary play through the ordinary keys.

| Key | Sets up |
| --- | --- |
| F1 to F5 | Rows about to clear. A hard drop removes 1 to 5 of them. |
| F6 | The next row cleared raises the level. |
| F7 | A well filled to the brim. The next lock ends the game. |
| F8 | An empty well, the piece back at the top. |

They are also the quickest way to hear a cue by hand:

```bash
cargo run --release --features stage      # then F4 and Space
```

The game as shipped has none of this: the feature is off by default, and
`record.sh` builds it into `target/stage/`, away from `target/release/`.

### Writing a session

A session is a shell file in `tools/sound/sessions/`. Its first line says what
it plays. Then:

```bash
# A four-row clear, twice.
# seconds: 8
# needs: stage
# expect: lock+hard_drop+clear_4 | lock+hard_drop+clear_4

rest 1.2
press F4;  rest 0.15;  press space;  rest 2.5
press F4;  rest 0.15;  press space;  rest 2.5
```

| Line | Meaning |
| --- | --- |
| `# seconds: N` | How long to record. Required. |
| `# needs: stage` | Build with the staging keys. |
| `# expect: a+b \| c` | The cues the game must play, and no others: groups in order, separated by `\|`; the cues of a group start together and are joined by `+`. `none` for silence. |
| `# expect-within: ...` | The same groups in the same order, but other cues may come between. |
| `# then: seam` | Run `bench.py seam` on the take as well. |
| `press <key>...` | Tap each key, 60 ms down. |
| `hold <key> <seconds>` | Keep a key down. |
| `rest <seconds>` | Wait. |

Keys go by their X names: `Left`, `Right`, `Up`, `Down`, `space`, `p`, `r`,
`F1` to `F8`. Start with a rest of a second, so that the theme has begun. The
game pauses itself after 8 seconds without a key, and a tap does not count for
long: in a long session, hold a key now and then, as `loop` does.

## What this does not tell you

- **How it sounds.** The numbers say whether a cue is there, how loud, and
  whether the mix has room. Whether it is the right sound is for ears:
  `bench.py listen`.
- **The delay between a key and its sound.** A timeline shows about 200 ms.
  Most of it is the virtual sound card, which is given a buffer of 150 ms so
  that it does not drop out. Read the order of things from a timeline, not
  the delay.
- **Other mixers.** The bench models macroquad's mixer on Linux and macOS,
  where a block is 4096 frames. On Windows its blocks vary in length.
- **Three or more events piling up.** `check` tries every single block and
  every pair of locks. Beyond that it samples.
- **Small speakers, exactly.** `heard` is an estimate from band levels. A
  sound that lives below 315 Hz reads worse there than it sounds on anything
  with a bass.

A recording that the virtual card dropped out of is not a verdict on the game.
`record.sh` notices, says `UNUSABLE` and records again, up to three times. If
it keeps happening, the machine is too busy.

## Files

| File | What it is |
| --- | --- |
| `bench.py` | The commands. |
| `record.sh` | The recorder. |
| `sessions/*.sh` | The sessions. |
| `keys.sh` | `press`, `hold` and `rest`. |
| `game.py` | Reads the levels and rules from `src/sound.rs`, and the files as the game decodes them. |
| `model.py` | The mixer, modelled: renders a mix without the game, and searches for its highest peak. |
| `recording.py` | Takes a recording apart. |
| `sig.py` | Audio files, levels, loudness, spectra, pictures. |
| `marker.py` | The click that puts keys and sound on one clock. |
| `asound.conf` | Points the game's sound at the take's virtual card. |
| `../../examples/decode.rs` | Decodes a sound file as the game does. |
| `../../src/stage.rs` | The staging keys. |
