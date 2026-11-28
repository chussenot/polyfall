"""What the game has: its sound files as its own decoder reads them, and the levels and rules
written in src/sound.rs.

Nothing is copied here. Levels, file names and timings are read out of the Rust source, so the
bench cannot drift from the game; if the source changes shape, it stops with a clear message.
Cues are named by their file: `move_left`, `clear_4`, `theme`.
"""
import re
import subprocess
from pathlib import Path

import sig

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "target" / "sound"      # everything the bench writes; target/ is not in git
BLOCK = 4096                          # frames macroquad's mixer mixes at a time
RATE = sig.RATE                       # the rate it mixes at
BLOCK_SECONDS = BLOCK / RATE
LOOPS = ("theme", "soft_drop")        # the cues the game plays round and round


def _source():
    return (ROOT / "src" / "sound.rs").read_text()


def _need(found, what):
    if not found:
        raise SystemExit(f"tools/sound is out of step with src/sound.rs: cannot find {what}")
    return found


def files():
    """{cue name: path of its file}, from the `asset!` lines of `Cue::bytes`."""
    pairs = _need(re.findall(r'Cue::(\w+) => asset!\("([^"]+)"\)', _source()), "the asset! table")
    return {Path(path).stem: ROOT / "assets" / path for _, path in pairs}


def balance():
    """{cue name: its number in the table of `fn level`}: the balance between the sounds."""
    src = _source()
    body = _need(re.search(r"fn level\(cue: Cue\) -> f32 \{(.*?)\n\}", src, re.S), "fn level")
    by_variant = dict(re.findall(r"Cue::(\w+) => ([\d.]+),", body.group(1)))
    stem = {variant: Path(path).stem for variant, path in re.findall(r'Cue::(\w+) => asset!\("([^"]+)"\)', src)}
    _need(set(by_variant) == set(stem), "a level for every cue")
    _need("balance * MASTER" in body.group(1), "`balance * MASTER` in fn level")
    return {stem[variant]: float(value) for variant, value in by_variant.items()}


def master():
    """The factor on every level: the room left for sounds to pile up."""
    return rules()["MASTER"]


def levels():
    """{cue name: level}: the volume the game plays the cue at, balance times master."""
    return {name: value * master() for name, value in balance().items()}


def rules():
    """The desk's timings and ducks, by the names they have in the source."""
    src = _source()
    out = {name: float(value) for name, value in re.findall(r"const (\w+): f32 = ([-\d.]+);", src)}
    ducks = _need(re.search(r"const CLEAR_DUCK: \[f32; 5\] = \[([^\]]+)\]", src), "CLEAR_DUCK")
    out["CLEAR_DUCK"] = [float(v) for v in ducks.group(1).split(",")]
    level_up = _need(re.search(r"const LEVEL_UP_DUCK: \(f32, f32\) = \(([^)]+)\)", src), "LEVEL_UP_DUCK")
    out["LEVEL_UP_DUCK"] = tuple(float(v) for v in level_up.group(1).split(","))
    for name in ("MASTER", "DUCK_HOLD", "DUCK_RELEASE", "MOVE_GAP"):
        _need(name in out, name)
    return out


_decoded = {}


def _notes():
    """{cue name: "channels rate frames"}: what the decoder said of each file in the cache."""
    path = OUT / "decoded" / "rates"
    if not path.exists():
        return {}
    return dict(line.split(None, 1) for line in path.read_text().splitlines() if line.strip())


def decoded():
    """{cue name: audio}, each file decoded as the game decodes it (examples/decode.rs).
    Cached under target/sound/decoded; a file is decoded again when it is newer than its copy."""
    if _decoded:
        return _decoded
    cache = OUT / "decoded"
    cache.mkdir(parents=True, exist_ok=True)
    paths, notes = files(), _notes()

    def fresh(name, path):
        copy = cache / f"{name}.f32"
        return name in notes and copy.exists() and copy.stat().st_mtime >= path.stat().st_mtime

    stale = [path for name, path in paths.items() if not fresh(name, path)]
    if stale:
        run = subprocess.run(["cargo", "run", "--quiet", "--release", "--example", "decode", "--", str(cache)]
                             + [str(path) for path in stale], cwd=ROOT, capture_output=True, text=True)
        if run.returncode:
            raise SystemExit("could not decode the sound files:\n" + run.stderr)
        notes.update(line.split(None, 1) for line in run.stdout.splitlines() if line.strip())
        (cache / "rates").write_text("".join(f"{name} {rest}\n" for name, rest in sorted(notes.items())))
    for name in paths:
        _decoded[name] = sig.read_f32(cache / f"{name}.f32")
    return _decoded


def rates():
    """{cue name: (channels, sample rate)} of the files, as the decoder reported them."""
    decoded()
    notes = _notes()
    return {name: tuple(int(v) for v in notes[name].split()[:2]) for name in files()}
