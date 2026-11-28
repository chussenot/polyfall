"""A model of the game's mixer, for trying a mix without running the game.

macroquad's mixer only adds: every playing sound times its volume, summed. This does the same
with the files as the game decodes them and the levels in src/sound.rs, but keeps the sum as it
is instead of clipping it at full scale, so too little headroom shows up as a number.

The theme is lowered under row clears and the level-up by the desk's own rules, and sounds start
on the mixer's block boundaries, as in the game. Recordings of the running game have matched
this model sample for sample.
"""
import numpy as np

import game
import sig


def theme_gain(events, seconds):
    """The factor on the theme for every sample: the desk's ducks, stepped once a block."""
    rules = game.rules()
    hold_clear, release = rules["DUCK_HOLD"], rules["DUCK_RELEASE"]
    blocks = int(np.ceil(seconds / game.BLOCK_SECONDS))
    out = np.ones(blocks)
    duck = hold = 0.0
    pending = sorted((t, name) for t, name, _ in events)
    for k in range(blocks):
        t = k * game.BLOCK_SECONDS
        while pending and pending[0][0] <= t:
            _, name = pending.pop(0)
            depth, length = 0.0, 0.0
            if name.startswith("clear_"):
                depth, length = rules["CLEAR_DUCK"][int(name[-1]) - 1], hold_clear
            elif name == "level_up":
                depth, length = rules["LEVEL_UP_DUCK"]
            if depth < 0:
                duck, hold = min(duck, depth), max(hold, length)
        out[k] = sig.gain(duck)
        if hold > 0:
            hold -= game.BLOCK_SECONDS
        else:
            duck = min(0.0, duck + release * game.BLOCK_SECONDS)
    return np.repeat(out, game.BLOCK)[:int(seconds * game.RATE)]


def render(events, seconds, theme_from=0.0, music=True):
    """The mix of `events` over the theme, not clipped.

    events: (seconds, cue name, factor on its level). `theme_from` is where in the theme the
    excerpt begins. With `music` false the theme is left out."""
    sounds, levels = game.decoded(), game.levels()
    n = int(seconds * game.RATE)
    out = np.zeros((n, 2))
    if music:
        theme = sounds["theme"]
        at = (int(round(theme_from * game.RATE)) + np.arange(n)) % len(theme)
        out += theme[at] * levels["theme"] * theme_gain(events, seconds)[:, None]
    for t, name, factor in events:
        start = int(np.ceil(t / game.BLOCK_SECONDS - 1e-9)) * game.BLOCK   # the next block boundary
        x = sounds[name] * levels[name] * factor
        end = min(n, start + len(x))
        if end > start:
            out[start:end] += x[:end - start]
    return out


def loudest_stretch(seconds=20.0):
    """Where the theme is loudest over `seconds`: (start in seconds, K-weighted level)."""
    theme = game.decoded()["theme"]
    levels = sig.window_levels(theme, seconds)
    hop = int(seconds * game.RATE) // 4
    k = int(np.argmax(levels))
    start = k * hop // game.BLOCK * game.BLOCK        # on a block, as everything in the game is
    return start / game.RATE, float(levels[k]) + sig.db(game.levels()["theme"])


# --- the highest peak the mix can reach ------------------------------------------------
#
# The mixer has no limiter, so the levels have to leave the room themselves. How much is left is
# not a matter of adding peaks up: two attacks can cancel as well as add, and the theme is only
# loud in places. But the mixer starts every sound on a block, and the theme is a whole number of
# blocks long. So a group of cues can only ever meet the theme in as many ways as the theme has
# blocks, about a thousand, and every one of them can be tried. That is what `highest` does.

LOCKS = ("lock", "hard_drop")          # a lock at full strength: the heaviest piece, the longest fall


def lock_events():
    """Everything one lock can start, at full strength: {label: [cue names]}. The lock with its
    hard drop; with one to five rows cleared; and those with a level-up."""
    out = {"lock + hard drop": list(LOCKS)}
    for rows in range(1, 6):
        out[f"lock + hard drop + {rows} row(s)"] = [*LOCKS, f"clear_{rows}"]
        out[f"lock + hard drop + {rows} row(s) + level up"] = [*LOCKS, f"clear_{rows}", "level_up"]
    return out


def one_block():
    """Every group of cues that can start on one block, so on the same sample: a lock event, and
    beside it any of a move, a turn and a refused key, pressed within the same 93 ms.
    {label: [(block, cue, factor)]}"""
    out = {}
    for label, cues in lock_events().items():
        for move in (None, "move_left", "move_right"):
            for turn in (None, "rotate"):
                for refusal in (None, "blocked"):
                    beside = [c for c in (move, turn, refusal) if c]
                    name = label + "".join(f" + {c.replace('_', ' ')}" for c in beside)
                    out[name] = [(0, cue, 1.0) for cue in cues + beside]
    return out


def two_locks(apart=range(1, 12)):
    """Two lock events, the second one to eleven blocks (up to a second) after the first, while
    the first still rings. {label: [(block, cue, factor)]}"""
    out = {}
    events = lock_events()
    for first, a in events.items():
        for second, b in events.items():
            if "level_up" in a and "level_up" in b:
                continue                 # ten rows lie between two level-ups
            for gap in apart:
                out[f"{first}; {gap * game.BLOCK_SECONDS * 1000:.0f} ms later, {second}"] = (
                    [(0, cue, 1.0) for cue in a] + [(gap, cue, 1.0) for cue in b])
    return out


def highest(groups):
    """The highest peak any of `groups` reaches over the theme, tried at every block of the theme.

    groups: {label: [(block, cue, factor)]}. Returns a dict: peak (dBFS), label and seconds
    (which group, and where in the theme it starts), over (how many placements pass full scale),
    placements (how many there are), alone (the highest peak of a group without the theme) and
    anywhere (the peak if the highest sample of a group met the highest sample of the theme:
    out of reach while sounds start on blocks, the bound where they do not).

    Exact, and quick because most cases are ruled out without being mixed: a block of a group
    whose own peak plus the theme's cannot beat the best found so far is skipped, and so is
    every block of the theme too quiet to."""
    sounds, levels = game.decoded(), game.levels()
    theme = (sounds["theme"] * levels["theme"]).astype(np.float32)
    if len(theme) % game.BLOCK:
        raise SystemExit("the theme is not a whole number of blocks long: it meets the other sounds differently "
                         "on every pass, and the mixer plays its loop point wrong. Fix its length first.")
    blocks = theme.reshape(-1, game.BLOCK, 2)
    theme_peaks = np.abs(blocks).max(axis=(1, 2))
    theme_peak = float(theme_peaks.max())
    scaled = {}

    def build(events):
        """The group mixed, as [block][frame][channel]."""
        frames = max(block * game.BLOCK + len(sounds[cue]) for block, cue, _ in events)
        count = -(-frames // game.BLOCK)
        mix = np.zeros((count * game.BLOCK, 2), np.float32)
        for block, cue, factor in events:
            if (cue, factor) not in scaled:
                scaled[cue, factor] = (sounds[cue] * (levels[cue] * factor)).astype(np.float32)
            x = scaled[cue, factor]
            mix[block * game.BLOCK:block * game.BLOCK + len(x)] += x
        return mix.reshape(count, game.BLOCK, 2)

    # first, what each block of each group could reach at most; no audio is kept
    cases = []                           # (bound, label, block of the group, its peak, the gain on the theme)
    alone = anywhere = 0.0
    for label, events in groups.items():
        parts = build(events)
        peaks = np.abs(parts).max(axis=(1, 2))
        gains = theme_gain([(block * game.BLOCK_SECONDS, cue, factor) for block, cue, factor in events],
                           len(parts) * game.BLOCK_SECONDS)[::game.BLOCK]
        alone = max(alone, float(peaks.max()))
        for j, (peak, gain_) in enumerate(zip(peaks, gains)):
            bound = float(peak) + theme_peak * float(gain_)
            anywhere = max(anywhere, bound)
            cases.append((bound, label, j, float(peak), float(gain_)))
    # then the cases that could be the highest, from the most promising down
    cases.sort(key=lambda c: -c[0])
    best, over, built = (0.0, None, 0), set(), (None, None)
    for bound, label, j, peak, gain_ in cases:
        floor = min(best[0], 1.0)
        if bound <= floor:
            break                        # nothing further down can be the highest, or pass full scale
        where = np.where(peak + gain_ * theme_peaks > floor)[0]
        if built[0] != label:
            built = (label, build(groups[label]))
        mixed = blocks[where] * np.float32(gain_)
        mixed += built[1][j]
        np.abs(mixed, out=mixed)
        peaks = mixed.max(axis=(1, 2))
        k = int(np.argmax(peaks))
        if peaks[k] > best[0]:
            best = (float(peaks[k]), label, int((where[k] - j) % len(blocks)))
        over.update((label, int((b - j) % len(blocks))) for b in where[peaks > 1.0])
    return dict(peak=sig.db(best[0]), label=best[1], seconds=best[2] * game.BLOCK_SECONDS, over=len(over),
                placements=len(groups) * len(blocks), alone=sig.db(alone), anywhere=sig.db(anywhere))


def play(seed, seconds=20.0):
    """A stretch of fast, busy play, made up: moves and turns between locks, most of them hard
    drops, a quarter of them clearing rows, now and then a level-up. Events for `render`."""
    import random
    rng, events, t = random.Random(seed), [], 1.0
    while t < seconds - 2:
        for _ in range(rng.randint(1, 4)):
            events.append((t, rng.choice(("move_left", "move_right", "rotate", "move_right", "move_left")), 1.0))
            t += rng.uniform(0.18, 0.32)
        if rng.random() < 0.1:
            events.append((t, "blocked", 1.0))
        t += rng.uniform(0.0, 0.25)
        events.append((t, "lock", rng.choice((0.84, 0.92, 1.0))))
        if rng.random() < 0.7:
            events.append((t, "hard_drop", rng.uniform(0.4, 1.0)))
        if rng.random() < 0.25:
            events.append((t, f"clear_{rng.choice((1, 1, 2, 3, 4, 5))}", 1.0))
            if rng.random() < 0.2:
                events.append((t, "level_up", 1.0))
        t += rng.uniform(0.2, 0.75)
    return events
