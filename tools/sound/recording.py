"""Takes a recording of the running game apart.

A take is a directory written by record.sh: the recording (out.wav, in floats, so that a sum over
full scale is kept as it was), the keys that were sent and when (keys), the session that sent
them (session.sh) and what the game printed (game.log).

The method rests on one fact: macroquad's mixer starts every sound on a block of 4096 samples.
So: find where the theme starts in the recording, then, block by block, fit the recording as
the theme times a volume, plus every cue that starts on that block times its own volume. What
comes out is the list of what the game actually played, when and how loud, next to the keys;
the theme's volume over time (ducks, pause fades); and whatever is left unexplained, which for
a healthy game is nothing but rounding.
"""
import re
from pathlib import Path

import numpy as np

import game
import sig

BLOCK = game.BLOCK
QUIET = 0.03          # a cue fitted below this volume is not there
MOST = 0.5            # a cue must explain this share of its own energy to be accepted
LAST = 0.25           # seconds before the game is stopped that are left out: the sound card still held them
FAINT = 1e-10         # mean square (-100 dBFS) under which a block of the theme is silence: no volume to read from it


class Take:
    def __init__(self, path):
        self.path = Path(path)
        if not (self.path / "out.wav").exists():
            raise SystemExit(f"{self.path}: not a take (no out.wav); record one with tools/sound/record.sh")
        self.audio, rate = sig.read_wav(self.path / "out.wav")
        assert rate == game.RATE
        lines = [line.split(None, 1) for line in (self.path / "keys").read_text().splitlines() if line.strip()]
        marks = [float(t) for t, what in lines if what == "MARKER"]
        loud = np.abs(self.audio[:2 * rate]).max(1) > 0.5           # the click comes before the game makes a sound
        if not marks or not loud.any():
            raise SystemExit(f"{self.path}: no marker click in the recording; the clocks cannot be matched")
        at = int(np.argmax(loud))
        # key time = recording time + offset
        self.offset = marks[0] - at / rate
        self.audio[max(0, at - 50):at + int(0.02 * rate)] = 0      # the click is not the game's
        # the recorder stops the game at END, in the middle of a block: what follows is not the game's either
        ends = [float(t) for t, what in lines if what == "END"]
        if ends:
            self.audio = self.audio[:max(0, int((ends[0] - self.offset - LAST) * rate))]
        self.keys = [(float(t), what) for t, what in lines if what not in ("MARKER", "END")]
        log = self.path / "game.log"
        self.dropouts = log.read_text().count("Underrun") if log.exists() else 0
        session = self.path / "session.sh"
        self.session = session.read_text() if session.exists() else ""

    def expectation(self):
        """(groups, exact) from the session's `# expect:` or `# expect-within:` line, or None."""
        found = re.search(r"^# (expect|expect-within):(.*)$", self.session, re.M)
        if not found:
            return None
        text = found.group(2).strip()
        groups = [] if text == "none" else [sorted(part.strip().split("+")) for part in text.split("|")]
        return groups, found.group(1) == "expect"


def _chunk(x, start, loop=False):
    """BLOCK frames of x from `start`, flattened; wrapped if it loops, else padded with silence."""
    if loop:
        return x[(start + np.arange(BLOCK)) % len(x)].ravel()
    part = x[start:start + BLOCK]
    if len(part) < BLOCK:
        part = np.vstack([part, np.zeros((BLOCK - len(part), 2))])
    return part.ravel()


def find_theme(audio, theme, within):
    """The first sample of the theme: the best lag of its first two seconds, looked for only
    in the first `within` seconds (it starts with the game; a loud effect later must not be
    mistaken for it)."""
    n = int(within * game.RATE)
    head = theme[:2 * game.RATE, 0]
    part = audio[:n + len(head), 0]
    size = 1 << int(np.ceil(np.log2(len(part) + len(head))))
    lags = np.fft.irfft(np.fft.rfft(part, size) * np.conj(np.fft.rfft(head, size)), size)[:n]
    return int(np.argmax(lags))


def _fit(y, columns, candidates):
    """Least squares of y on `columns`, then on any candidate that keeps earning its place.
    Returns ({name: volume}, what is left of y)."""
    def solve(extra):
        cols = [c for _, c in columns] + [c for _, c in extra]
        if not cols:
            return np.zeros(0), y, float((y ** 2).sum())
        m = np.array(cols).T
        g = np.linalg.lstsq(m, y, rcond=None)[0]
        rest = y - m @ g
        return g, rest, float((rest ** 2).sum())

    extra = []
    g, rest, err = solve(extra)
    while len(extra) < 6:
        best = None
        for name, head in candidates.items():
            if any(name == n for n, _ in extra):
                continue
            g2, _, err2 = solve(extra + [(name, head)])
            volume = g2[-1]
            if volume < QUIET or err - err2 < MOST * volume * volume * float((head ** 2).sum()):
                continue
            if best is None or err - err2 > best[0]:
                best = (err - err2, name, head)
        if best is None or best[0] < 1e-3:
            break
        extra.append((best[1], best[2]))
        g, rest, err = solve(extra)
    return dict(zip([n for n, _ in columns] + [n for n, _ in extra], g)), rest


def take_apart(take):
    """What the game played. Returns a dict:
      theme_at   key time of the theme's first sample
      cues       [(key time, block number, cue name, volume)]
      loop       [(start, end or None)] of the soft-drop loop
      theme      volume of the theme per block (nan where it is too quiet to tell)
      left       what theme and cues do not explain, as audio
      worst      the level of that in its worst block, dBFS
      lost       key time at which the recording stopped matching the mixer's blocks, or None
    """
    sounds = dict(game.decoded())
    theme, run_loop = sounds.pop("theme"), sounds.pop("soft_drop")
    audio, rate, offset = take.audio, game.RATE, take.offset
    first_key = min([t for t, _ in take.keys] + [len(audio) / rate + offset]) - offset
    p0 = find_theme(audio, theme, max(1.0, first_key + 0.3))
    blocks = (len(audio) - p0) // BLOCK
    heads = {name: _chunk(x, 0) for name, x in sounds.items()}
    heads["soft_drop"] = _chunk(run_loop, 0)

    origin, theme_on, run = p0, True, None      # run: (first sample, volume) of a playing soft-drop loop
    ringing = []                                # one-shots still sounding: [name, first sample, volume]
    cues, loops, volumes = [], [], []
    left = np.zeros((blocks * BLOCK, 2))
    astray, lost = 0, None

    for k in range(blocks):
        pos = p0 + k * BLOCK
        t = pos / rate + offset
        raw = audio[pos:pos + BLOCK].ravel()
        for _ in range(4):          # a cue that silences others changes what this block holds: look again
            y = raw.copy()
            for name, start, volume in ringing:
                if pos - start < len(sounds[name]):
                    y -= volume * _chunk(sounds[name], pos - start)
            columns = []
            ours = _chunk(theme, (pos - origin) % len(theme), loop=True)
            readable = theme_on and (ours ** 2).mean() > FAINT
            if readable:
                columns.append(("theme", ours))
            if run is not None:
                columns.append(("~run", _chunk(run_loop, (pos - run[0]) % len(run_loop), loop=True)))
            candidates = dict(heads)
            if run is not None:
                candidates.pop("soft_drop")
            if not (theme_on and (pos - origin) % len(theme) == 0):
                candidates["theme!"] = _chunk(theme, 0)         # the theme starting right here: a new game
            found, rest = _fit(y, columns, candidates)
            if run is not None and found.get("~run", 0) < 0.5 * run[1]:
                loops.append((run[0] / rate + offset, t))
                run = None
                continue
            if (found.get("new_game", 0) > 0.1 or found.get("pause", 0) > 0.1) and (ringing or run is not None):
                if run is not None:
                    loops.append((run[0] / rate + offset, t))
                    run = None
                ringing = []                                    # both stop every effect
                continue
            if found.get("theme!", 0) > 0.2 and (origin != pos or not theme_on):
                origin, theme_on = pos, True
                continue
            if found.get("game_over", 0) > 0.1 and (theme_on or any(n == "level_up" for n, _, _ in ringing)):
                theme_on = False                                # the ending stops the theme and the level-up
                ringing = [r for r in ringing if r[0] != "level_up"]
                continue
            break
        volumes.append((found.get("theme", float("nan")) if readable else float("nan")) if theme_on else 0.0)
        for name, volume in found.items():
            if name in ("theme", "~run", "theme!"):
                continue
            if name == "soft_drop":
                run = (pos, volume)
            else:
                ringing.append([name, pos, volume])
            cues.append((t, k, name, float(volume)))
        ringing = [r for r in ringing if pos + BLOCK - r[1] < len(sounds[r[0]])]
        left[k * BLOCK:(k + 1) * BLOCK] = rest.reshape(-1, 2)
        # a dropout of the virtual sound card shifts everything after it off the block grid
        energy = float((raw ** 2).sum())
        astray = astray + 1 if energy > 1e-2 and float((rest ** 2).sum()) > 0.5 * energy else 0
        if astray >= 5:
            lost = (p0 + (k - 4) * BLOCK) / rate + offset
            cues = [c for c in cues if c[0] < lost]
            volumes = volumes[:k - 4]
            left = left[:(k - 4) * BLOCK]
            break
    if run is not None:
        loops.append((run[0] / rate + offset, None))
    worst = max((sig.rms(left[k:k + BLOCK]) for k in range(0, len(left), BLOCK)), default=-180.0)
    return dict(theme_at=p0 / rate + offset, p0=p0, cues=cues, loop=loops, theme=np.array(volumes),
                left=left, worst=worst, lost=lost)


def groups(result):
    """The cues played, grouped by the block they started on: [[names, sorted], ...] in order."""
    out, last = [], None
    for _, block, name, _ in sorted(result["cues"], key=lambda c: (c[1], c[2])):
        if block != last:
            out.append([])
            last = block
        out[-1].append(name)
    return [sorted(g) for g in out]


def timeline(take, result):
    """One line per key, cue and change of the theme's volume, in time order."""
    events = [(t, 0, f"KEY   {what}") for t, what in take.keys]
    events += [(t, 2, f"  cue {name:<10} volume {volume:.3f}") for t, _, name, volume in result["cues"]]
    events += [(end, 2, "  cue soft_drop  stops") for _, end in result["loop"] if end is not None]
    last = None
    for k, volume in enumerate(result["theme"]):
        if volume == volume and (last is None or abs(volume - last) >= 0.015):
            events.append(((result["p0"] + k * BLOCK) / game.RATE + take.offset, 1, f"      theme volume {abs(volume):.2f}"))
            last = volume
    return [f"{t:8.3f}  {what}" for t, _, what in sorted(events)]


def matches(expected, exact, played):
    """Whether the cue groups that were played fit the session's expectation."""
    if exact:
        return played == expected
    at = 0
    for want in expected:
        while at < len(played) and not set(want) <= set(played[at]):
            at += 1
        if at == len(played):
            return False
        at += 1
    return True


def seam(take):
    """What the recording holds at the theme's loop point. Aligns on the theme's last second, so an
    earlier dropout does not matter, then compares what follows with a correct loop and with what
    the mixer does to a loop that ends inside a block. Returns a dict of facts, or None if the take
    does not reach the loop point."""
    theme = game.decoded()["theme"]
    volume = game.levels()["theme"]
    audio, rate, n = take.audio, game.RATE, len(theme)
    p0 = find_theme(audio, theme, 3.0)
    whole, tail_frames = divmod(n, BLOCK)
    if p0 + n + 3 * BLOCK > len(audio):
        return None
    before, after = int(1.2 * rate), int(0.2 * rate)
    template = theme[n - before:n - after, 0] * volume
    low = p0 + n - before - rate
    part = audio[low:low + 2 * rate + len(template), 0]
    size = 1 << int(np.ceil(np.log2(len(part) + len(template))))
    lags = np.fft.irfft(np.fft.rfft(part, size) * np.conj(np.fft.rfft(template, size)), size)[:2 * rate]
    start = low + int(np.argmax(lags))
    end = start + before                        # where the theme's last frame + 1 lands
    aligned = sig.rms(audio[start:start + len(template)] - volume * theme[n - before:n - after])
    heard = audio[end - tail_frames:end + 2 * BLOCK] / volume
    correct = np.vstack([theme[whole * BLOCK:], theme[:len(heard) - tail_frames]])
    facts = dict(frames=n, blocks=whole, extra=tail_frames, aligned=aligned,
                 against_correct=sig.rms(heard - correct), level=sig.rms(correct), excerpt=audio[end - rate:end + rate])
    if tail_frames:
        early = BLOCK - tail_frames
        wrong = np.zeros_like(heard)
        wrong[:tail_frames] += theme[whole * BLOCK:]
        wrong[:early] += theme[:early]
        wrong[BLOCK:] = theme[early:early + len(heard) - BLOCK]
        facts.update(against_mixer_bug=sig.rms(heard - wrong), hole=sig.rms(heard[tail_frames:BLOCK]),
                     hole_should_be=sig.rms(correct[tail_frames:BLOCK]))
    return facts
