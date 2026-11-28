#!/usr/bin/env python3
"""The sound bench: measures Polyfall's sounds and mix, and takes recordings of the game apart.

  bench.py check              the files and the room in the mix, pass or fail (no sound card needed)
  bench.py sheet              every cue as the game plays it: peak, loudness, loudness against the theme
  bench.py heard              how far each effect stands above the level at which the theme covers it
  bench.py spec [cue...]      a spectrogram of each cue, as PNG
  bench.py layers cue...      the balance of the layers inside an effect   (needs sonomorph-render)
  bench.py trim [--write]     the TRIM that brings each effect to -3 dBFS  (needs sonomorph-render)
  bench.py take <take>        what the game played in a recording, and whether that was right
  bench.py seam <take>        what a recording holds at the theme's loop point
  bench.py listen             files to listen to: every sound alone, the mix over the loudest bars, the takes

A take is made by tools/sound/record.sh. Everything is written under target/sound/.
See tools/sound/README.md.
"""
import os
import re
import shutil
import signal
import subprocess
import sys

import numpy as np

import game
import model
import recording
import sig

EFFECT_PEAK = -3.0        # dBFS every effect file and the level-up are brought to
MUSIC = ("theme", "level_up", "game_over")
ORDER = ["soft_drop", "move_left", "move_right", "rotate", "blocked", "lock", "hard_drop",
         "clear_1", "clear_2", "clear_3", "clear_4", "clear_5", "pause", "resume", "new_game",
         "level_up", "game_over"]
AUDIBLE = [315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300]   # Hz: what a laptop plays


def sounding(x, floor_db=-60.0):
    """(first, last + 1) frame above the floor, relative to full scale."""
    above = np.where(np.abs(x).max(1) > sig.gain(floor_db))[0]
    return (int(above[0]), int(above[-1]) + 1) if len(above) else (0, 0)


def ordered(names):
    return [n for n in ["theme"] + ORDER if n in names] + sorted(set(names) - set(ORDER) - {"theme"})


# --- check ----------------------------------------------------------------------------

def check_files():
    """Every file as the game decodes it. Returns the list of failures."""
    sounds, rates, failures = game.decoded(), game.rates(), []
    print(f"{'file':<11} {'frames':>8} {'seconds':>8} {'blocks':>9} {'peak':>7} {'starts':>8}")
    for name in ordered(sounds):
        x = sounds[name]
        channels, rate = rates[name]
        m = np.abs(x).max(1)
        starts = int(np.argmax(m > m.max() * 0.01)) / game.RATE * 1000
        blocks = len(x) / game.BLOCK
        print(f"{name:<11} {len(x):>8} {len(x) / game.RATE:>8.3f} {blocks:>9.3f} {sig.peak(x):>7.1f} {starts:>6.1f}ms")
        if rate != game.RATE:
            failures.append(f"{name}: {rate} Hz. The mixer runs at {game.RATE} and resamples anything else by nearest neighbour.")
        if name in game.LOOPS and len(x) % game.BLOCK:
            failures.append(f"{name}: a loop of {len(x)} frames is not a whole number of {game.BLOCK}-frame blocks "
                            f"({len(x) % game.BLOCK} frames over). The mixer will play the start of each pass early and leave a hole.")
        if name not in ("theme", "game_over") and abs(sig.peak(x) - EFFECT_PEAK) > 0.5:
            failures.append(f"{name}: peaks at {sig.peak(x):.1f} dBFS, not {EFFECT_PEAK:.0f}. Its level in src/sound.rs assumes {EFFECT_PEAK:.0f}; run `bench.py trim`.")
        if name not in MUSIC and starts > 5:
            failures.append(f"{name}: silent for its first {starts:.0f} ms. The mixer already adds a delay of its own.")
    return failures


def check_room():
    """The room at the top of the mix. The mixer has no limiter: a sum over full scale is
    clipped by the sound card. Returns the list of failures."""
    levels, sounds = game.levels(), game.decoded()
    theme = sounds["theme"] * levels["theme"]
    master = game.master()
    print(f"\nmaster {master:.2f}: every level is {-sig.db(master):.1f} dB under its number in the balance")
    print(f"the theme alone                              peak {sig.peak(theme):+6.1f} dBFS")
    failures = []
    for title, groups in (("all that can start on one block", model.one_block()),
                          ("two locks within a second", model.two_locks())):
        found = model.highest(groups)
        print(f"{title:<44} peak {found['peak']:+6.1f} dBFS   {found['over']} of {found['placements']} placements over full scale")
        print(f"    highest: {found['label']}, {found['seconds']:.1f} s into the theme")
        print(f"    without the theme {found['alone']:+.1f} dBFS; if its highest sample met the theme's, {found['anywhere']:+.1f} dBFS")
        if found["over"]:
            failures.append(f"{title}: passes full scale in {found['over']} placements, by up to {found['peak']:.1f} dB "
                            f"({found['label']}). The sound card will clip it: lower MASTER in src/sound.rs, or the "
                            f"level of what is in the pile.")
    seconds = len(theme) / game.RATE
    worst, over, runs = -180.0, 0, 12
    for seed in range(runs):
        x = model.render(model.play(seed, seconds), seconds)
        worst = max(worst, sig.peak(x))
        over += int((np.abs(x) > 1).any(1).sum())
    print(f"{runs} times through the theme at busy play        peak {worst:+6.1f} dBFS   {over} samples over full scale")
    if over:
        failures.append(f"busy play passes full scale ({over} samples in {runs} times through the theme): lower MASTER in src/sound.rs.")
    return failures


def cmd_check(_args):
    failures = check_files() + check_room()
    print()
    for f in failures:
        print("FAIL  " + f)
    print("check: " + (f"{len(failures)} problem(s)" if failures else "all good"))
    return 1 if failures else 0


# --- sheet ----------------------------------------------------------------------------

def cmd_sheet(_args):
    balance, levels, sounds = game.balance(), game.levels(), game.decoded()
    theme = sounds["theme"] * levels["theme"]
    moments = sig.window_levels(theme, 0.4)
    ordinary = float(np.median(moments))
    print(f"master {game.master():.2f}: every cue plays {-sig.db(game.master()):.1f} dB under its balance")
    print(f"theme, balance {balance['theme']:.2f}: peak {sig.peak(theme):.1f} dBFS; loudness, an ordinary moment {ordinary:.1f}, "
          f"a quiet one {np.percentile(moments, 10):.1f}, a loud one {np.percentile(moments, 90):.1f}")
    print(f"{'cue':<11} {'balance':>8} {'sounds for':>11} {'peak':>7} {'loudest':>8} {'against theme':>14}")
    for name in ordered(sounds)[1:]:
        x = sounds[name] * levels[name]
        _, end = sounding(x)
        loud = sig.loudest(x, 0.4 if name in MUSIC else 0.1)
        print(f"{name:<11} {balance[name]:>8.2f} {end / game.RATE:>10.2f}s {sig.peak(x):>7.1f} {loud:>8.1f} {loud - ordinary:>+14.1f}")
    both = model.render([(0, "lock", 1.0), (0, "hard_drop", 1.0)], 0.5, music=False)
    loud = sig.loudest(both)
    print(f"{'lock+hard':<11} {'':>8} {'':>11} {sig.peak(both):>7.1f} {loud:>8.1f} {loud - ordinary:>+14.1f}")
    print("\nbalance: the cue's number in `level()` in src/sound.rs. The rest is measured as the game plays it, at\n"
          "balance times master. sounds for: until it is 60 dB under full scale. peak: dBFS. loudest: its loudest\n"
          "100 ms, K-weighted (400 ms for the music cues). against theme: that, minus an ordinary moment of the theme.")
    return 0


# --- heard ----------------------------------------------------------------------------

def cmd_heard(_args):
    """Below a certain level the theme covers an effect. Two estimates of that level, and where
    each effect stands against the better of them."""
    levels, sounds = game.levels(), game.decoded()
    theme = sounds["theme"] * levels["theme"]
    bands = sig.band_frames(theme, AUDIBLE, hop=512)
    band_ordinary, band_loud = np.median(bands, 1), np.percentile(bands, 90, 1)
    clicks = sig.short_levels(sig.bandpass(theme, 1000, 8000))
    click_ordinary, click_loud = float(np.median(clicks)), float(np.percentile(clicks, 90))
    print(f"{'cue':<11} {'balance dB':>10} {'floor':>7} {'over it':>8} {'over it in loud bars':>21}")
    for name in ordered(sounds)[1:]:
        if name in MUSIC:
            continue
        x = sounds[name]
        tone = sig.band_frames(x, AUDIBLE, hop=128).max(1)
        tick = float(sig.short_levels(sig.bandpass(np.vstack([x, np.zeros((game.RATE // 10, 2))]), 1000, 8000)).max())

        def floor(band_ref, click_ref):
            by_band = 6 - np.sort(tone - band_ref)[-2]      # two bands 6 dB over the theme
            by_click = click_ref + 10 - tick                # or 10 dB over it for 5 ms, between 1 and 8 kHz
            return min(by_band, by_click)

        # the master lowers theme and effect alike, so it drops out: everything here is in balance terms
        level = sig.db(levels[name])
        ordinary, loud = floor(band_ordinary, click_ordinary), floor(band_loud, click_loud)
        up = -sig.db(game.master())
        print(f"{name:<11} {level + up:>10.1f} {ordinary + up:>7.1f} {level - ordinary:>+8.1f} {level - loud:>+21.1f}")
    print("\nbalance dB: the cue's number in `level()`, in decibels. floor: the balance at which the effect is plainly\n"
          "heard over an ordinary moment of the theme on small speakers, which play 315 Hz to 6.3 kHz. A tonal\n"
          "effect needs two bands 6 dB above the theme; a tick needs to stand 10 dB above it for 5 ms. Negative\n"
          "'over it' means partly covered there. A sound that lives below 315 Hz (blocked, lock) reads worse here\n"
          "than it sounds on anything with a bass. An estimate: it says which way to move a level, not where to\n"
          "stop. Ears decide.")
    return 0


# --- spec -----------------------------------------------------------------------------

def cmd_spec(args):
    sounds = game.decoded()
    names = args or [n for n in ordered(sounds) if n not in ("theme", "game_over")]
    out = game.OUT / "spec"
    out.mkdir(parents=True, exist_ok=True)
    for name in names:
        if name not in sounds:
            raise SystemExit(f"no cue called {name}; there are: {' '.join(ordered(sounds))}")
        x = sounds[name]
        _, end = sounding(x)
        seconds = next(s for s in (0.4, 0.9, 2.2, 5.0, 15.0, 120.0) if s >= end / game.RATE)
        sig.write_png(out / f"{name}.png", sig.spectrogram(x, seconds))
        print(f"{out / (name + '.png')}   0 to {seconds:g} s")
    print("time left to right; 50 Hz at the bottom to 16 kHz at the top; blue lines at 100 Hz, 1 kHz, 10 kHz")
    return 0


# --- layers and trim: these render scores, so they need sonomorph-render ---------------

def renderer():
    tool = os.environ.get("SONOMORPH_RENDER") or shutil.which("sonomorph-render")
    if not tool:
        raise SystemExit("this needs sonomorph-render on the PATH (or SONOMORPH_RENDER=/path/to/it); see audio/README.md")
    return tool


def render_score(text, tool):
    """Renders score text; returns the audio before any clipping (raw floats)."""
    work = game.OUT / "work"
    work.mkdir(parents=True, exist_ok=True)
    (work / "it.score").write_text(text)
    run = subprocess.run([tool, str(work / "it.score"), str(work / "it.f32"), str(game.RATE)], capture_output=True, text=True)
    if run.returncode:
        raise SystemExit("sonomorph-render failed:\n" + run.stderr)
    return sig.read_f32(work / "it.f32")


def score_of(name):
    for path in (game.ROOT / "audio" / "sfx" / f"{name}.score", game.ROOT / "audio" / f"{name}.score"):
        if path.exists():
            return path
    raise SystemExit(f"no score called {name} in audio/ or audio/sfx/")


def cmd_layers(args):
    if not args:
        raise SystemExit("usage: bench.py layers <cue>...")
    tool = renderer()
    for name in args:
        lines = score_of(name).read_text().split("\n")
        layer = lambda inst: re.sub(r"\d+$", "", inst)
        layers = sorted({layer(line.split()[3]) for line in lines if line.startswith("note ")})
        print(name)
        for which in layers + [None]:
            keep = [line for line in lines if not line.startswith("note ") or which is None or layer(line.split()[3]) == which]
            x = render_score("\n".join(keep), tool)
            print(f"  {which or 'all together':<14} peak {sig.peak(x):>6.1f} dBFS   loudest 100 ms {sig.loudest(x):>6.1f}")
    print("\nrendered at the score's own gain, before clipping: read the layers against each other, not against the game")
    return 0


def cmd_trim(args):
    tool = renderer()
    script = game.ROOT / "audio" / "sfx.py"
    source = script.read_text()
    start = source.index("TRIM = {")
    end = source.index("}", start) + 1
    current = {k: float(v) for k, v in re.findall(r'"(\w+)": (-?[\d.]+)', source[start:end])}
    wanted = {}
    for name in current:
        x = render_score(score_of(name).read_text(), tool)
        wanted[name] = round(current[name] + EFFECT_PEAK - sig.peak(x), 1)
        print(f"{name:<11} peaks at {sig.peak(x):>6.1f} dBFS with trim {current[name]:>5.1f}   ->   trim {wanted[name]:>5.1f}")
    names = list(current)
    rows = [names[i:i + 4] for i in range(0, len(names), 4)]
    table = "TRIM = {\n" + "".join("    " + " ".join(f'"{n}": {wanted[n]},' for n in row) + "\n" for row in rows) + "}"
    if "--write" in args:
        script.write_text(source[:start] + table + source[end:])
        print(f"\nwritten to {script}. Now: python3 audio/sfx.py && mise run audio")
    else:
        print("\n" + table + "\n\npaste this into audio/sfx.py, or run `bench.py trim --write`")
    return 0


# --- take and seam: recordings of the running game -------------------------------------

def take_path(arg):
    for path in (game.OUT / "takes" / arg, game.ROOT / arg):
        if (path / "out.wav").exists():
            return path
    raise SystemExit(f"no take called {arg}; record one with tools/sound/record.sh")


UNUSABLE = 3           # exit status for a recording that cannot be judged; record.sh records it again
UNEXPLAINED = -60.0    # dBFS: a block of the recording that theme and cues leave more than this of is a problem


def cmd_take(args):
    if not args:
        raise SystemExit("usage: bench.py take <take>")
    take = recording.Take(take_path(args[0]))
    result = recording.take_apart(take)
    print(f"theme starts at {result['theme_at']:.3f} s; the mixer's blocks are {game.BLOCK_SECONDS * 1000:.1f} ms")
    print("\n".join(recording.timeline(take, result)))
    played = recording.groups(result)
    heard = take.audio[result["p0"]:]
    clipped = int((np.abs(heard) > 1.0).any(1).sum())
    print(f"\nrecording: peak {sig.peak(heard):+.2f} dBFS, {clipped} samples over full scale")
    print(f"not explained by the theme and the cues: {sig.rms(result['left']):.1f} dBFS overall, "
          f"{result['worst']:.1f} dBFS in the worst block")
    if take.dropouts:
        print(f"the virtual sound card dropped out {take.dropouts} time(s): the game was starved of CPU")
    if result["lost"] is not None:
        print(f"UNUSABLE  the recording stops matching the mixer's blocks at {result['lost']:.2f} s: the virtual sound "
              f"card dropped out. This says nothing about the game; record again, on a quieter machine if it keeps happening.")
        return UNUSABLE
    problems = []
    if clipped:
        problems.append("the mix passed full scale: a sound card clips that")
    levels = game.levels()
    for _, _, name, volume in result["cues"]:
        if volume > levels[name] * 1.02:
            problems.append(f"{name} played at {volume:.3f}, above its level of {levels[name]:.3f}. `bench.py check` "
                            f"takes a cue's level for the loudest it gets: its verdict on the room in the mix no longer holds.")
    if result["worst"] > UNEXPLAINED:
        problems.append("the game played something the theme and the cues do not account for")
    expectation = take.expectation()
    if expectation is not None:
        expected, exact = expectation
        show = lambda gs: " | ".join("+".join(g) for g in gs) or "none"
        if not recording.matches(expected, exact, played):
            problems.append(f"expected {'exactly' if exact else 'at least'}: {show(expected)}\n      played:   {show(played)}")
        else:
            print(f"cues: as the session expects ({show(expected)})")
    for p in problems:
        print("FAIL  " + p)
    print("take: " + (f"{len(problems)} problem(s)" if problems else "all good"))
    return 1 if problems else 0


def cmd_seam(args):
    if not args:
        raise SystemExit("usage: bench.py seam <take>")
    take = recording.Take(take_path(args[0]))
    facts = recording.seam(take)
    if facts is None:
        raise SystemExit("the recording does not reach the theme's loop point; record the `loop` session")
    print(f"theme: {facts['frames']} frames = {facts['blocks']} blocks + {facts['extra']} frames")
    print(f"aligned on its last second: the recording differs from it by {facts['aligned']:.1f} dBFS there")
    print(f"after the loop point, against a correct loop: differs by {facts['against_correct']:.1f} dBFS (the theme is at {facts['level']:.1f})")
    sig.write_wav(take.path / "seam.wav", facts["excerpt"] / game.levels()["theme"] * 0.9)
    print(f"one second either side of it: {take.path / 'seam.wav'}")
    if facts["extra"]:
        ms = (game.BLOCK - facts["extra"]) / game.RATE * 1000
        print(f"against what the mixer does to a loop that ends inside a block: differs by {facts['against_mixer_bug']:.1f} dBFS")
        print(f"the {ms:.1f} ms where the first notes belong: {facts['hole']:.1f} dBFS in the recording, {facts['hole_should_be']:.1f} dBFS in a correct loop")
    good = facts["against_correct"] < facts["level"] - 40
    print("seam: " + ("the loop is clean" if good else "FAIL  the loop is not clean; `bench.py check` says whether its length is a whole number of blocks"))
    return 0 if good else 1


# --- listen ---------------------------------------------------------------------------

def save(name, x):
    out = game.OUT / "listen"
    out.mkdir(parents=True, exist_ok=True)
    wav = out / f"{name}.wav"
    sig.write_wav(wav, x)
    if shutil.which("ffmpeg"):
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(wav), "-c:a", "libmp3lame", "-q:a", "2",
                        str(out / f"{name}.mp3")], check=True)
        wav.unlink()
        wav = out / f"{name}.mp3"
    print(f"{wav}   {len(x) / game.RATE:.1f} s, peak {sig.peak(x):.1f} dBFS")


def cmd_listen(_args):
    sounds = game.decoded()
    loop = len(sounds["soft_drop"]) / game.RATE
    # 1. every sound alone, at its level in the game
    events, notes, t = [], [], 0.3

    def put(name, factor=1.0, gap=0.9, also=(), say=None):
        nonlocal t
        events.append((t, name, factor))
        events.extend((t, other, 1.0) for other in also)
        notes.append(f"{t:6.2f} s  {say or name}")
        t += gap

    for name in ("move_left", "move_left", "move_right", "move_right"):
        put(name, gap=0.3)
    t += 0.5
    put("rotate", gap=0.35)
    put("rotate", gap=0.85)
    put("blocked")
    notes.append(f"{t:6.2f} s  soft_drop, three times round")
    events.extend((t + k * loop, "soft_drop", 1.0) for k in range(3))
    t += 3 * loop + 0.05
    # the factors of a small piece and of a short fall are typical values, not read from the game
    put("lock", 0.84, say="lock, a small piece (0.84 of its level)")
    put("lock", 1.0, say="lock, the biggest piece")
    put("hard_drop", 0.37, also=("lock",), say="hard drop, a short fall (0.37 of its level)")
    put("hard_drop", 1.0, gap=1.3, also=("lock",), say="hard drop, the longest fall")
    for n in range(1, 6):
        put(f"clear_{n}", gap=(1.3, 1.3, 1.5, 2.2, 2.8)[n - 1], also=("lock",), say=f"clear_{n}, with its lock")
    put("pause")
    put("resume")
    put("new_game", gap=1.2)
    put("level_up", gap=4.6)
    put("game_over", gap=12.0)
    save("every_sound_alone", model.render(events, t, music=False))
    (game.OUT / "listen" / "every_sound_alone.txt").write_text("\n".join(notes) + "\n")
    # 2. busy play over the theme's loudest bars, in the model of the mixer
    start, _ = model.loudest_stretch()
    save("over_the_loudest_bars", model.render(model.play(4), 20.0, theme_from=start))
    # 3. the takes
    takes = game.OUT / "takes"
    for path in sorted(takes.iterdir()) if takes.exists() else []:
        if (path / "out.wav").exists():
            take = recording.Take(path)
            save(f"take_{path.name}", take.audio)
    return 0


COMMANDS = {"check": cmd_check, "sheet": cmd_sheet, "heard": cmd_heard, "spec": cmd_spec, "layers": cmd_layers,
            "trim": cmd_trim, "take": cmd_take, "seam": cmd_seam, "listen": cmd_listen}

if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] not in COMMANDS:
        print(__doc__)
        sys.exit(2)
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)      # `bench.py sheet | head` ends quietly
    sys.exit(COMMANDS[sys.argv[1]](sys.argv[2:]))
