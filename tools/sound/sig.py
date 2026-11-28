"""Signals: reading and writing audio, levels, loudness, spectra, pictures.

Needs numpy and nothing else. Audio is a float array of shape (frames, 2), full scale at 1.0.
"""
import struct
import wave
import zlib

import numpy as np

RATE = 44100


def db(x):
    """A linear amplitude as decibels; silence reads -180."""
    return 20 * np.log10(max(float(x), 1e-9))


def gain(decibels):
    return 10 ** (decibels / 20)


def peak(x):
    """The highest sample, in dBFS."""
    return db(np.abs(x).max()) if len(x) else -180.0


def rms(x):
    return db(np.sqrt((x ** 2).mean())) if len(x) else -180.0


def read_wav(path):
    """Reads a WAV of 16-bit integers or 32-bit floats: (audio, sample rate). A float file can
    hold samples over full scale, which is the point of recording in floats."""
    with open(str(path), "rb") as f:
        data = f.read()
    assert data[:4] == b"RIFF" and data[8:12] == b"WAVE", f"{path}: not a WAV file"
    at, form, x = 12, None, None
    while at + 8 <= len(data):
        kind, size = data[at:at + 4], struct.unpack("<I", data[at + 4:at + 8])[0]
        body = data[at + 8:at + 8 + size]
        if kind == b"fmt ":
            tag, channels, rate, _, _, bits = struct.unpack("<HHIIHH", body[:16])
            if tag == 0xFFFE:                      # "extensible": the real tag is further in
                tag = struct.unpack("<H", body[24:26])[0]
            form = (tag, bits)
        elif kind == b"data":
            assert form in ((1, 16), (3, 32)), f"{path}: 16-bit or 32-bit float WAV expected"
            x = np.frombuffer(body, dtype="<i2" if form[0] == 1 else "<f4").reshape(-1, channels)
            x = x / 32768.0 if form[0] == 1 else x.astype(np.float64)
        at += 8 + size + (size & 1)
    assert x is not None, f"{path}: no audio in it"
    if x.shape[1] == 1:
        x = np.repeat(x, 2, axis=1)
    return x, rate


def write_wav(path, x, rate=RATE, floats=False):
    """Writes stereo audio as a WAV: 16-bit integers, clipped at full scale, for listening; or
    with `floats` 32-bit floats, exactly as given, for measuring."""
    if not floats:
        with wave.open(str(path), "wb") as w:
            w.setnchannels(2)
            w.setsampwidth(2)
            w.setframerate(rate)
            w.writeframes((np.clip(x, -1, 1) * 32767).astype("<i2").tobytes())
        return
    body = np.asarray(x, dtype="<f4").tobytes()
    with open(str(path), "wb") as f:
        f.write(b"RIFF" + struct.pack("<I", 50 + len(body)) + b"WAVE"
                + b"fmt " + struct.pack("<IHHIIHHH", 18, 3, 2, rate, rate * 8, 8, 32, 0)
                + b"fact" + struct.pack("<II", 4, len(body) // 8)
                + b"data" + struct.pack("<I", len(body)) + body)


def read_f32(path):
    """Interleaved stereo floats, as examples/decode.rs writes them."""
    return np.fromfile(str(path), dtype="<f4").reshape(-1, 2).astype(np.float64)


# --- loudness -------------------------------------------------------------------------
# K-weighting, the pre-filter of ITU-R BS.1770: a high shelf and a high-pass. It lets a low
# thump and a high tick be compared by something closer to how loud they are heard.

def _shelf(rate, f0=1681.974450955533, g=3.999843853973347, q=0.7071752369554196):
    a = 10 ** (g / 40)
    w0 = 2 * np.pi * f0 / rate
    alpha = np.sin(w0) / (2 * q)
    b = [a * ((a + 1) + (a - 1) * np.cos(w0) + 2 * np.sqrt(a) * alpha),
         -2 * a * ((a - 1) + (a + 1) * np.cos(w0)),
         a * ((a + 1) + (a - 1) * np.cos(w0) - 2 * np.sqrt(a) * alpha)]
    den = [(a + 1) - (a - 1) * np.cos(w0) + 2 * np.sqrt(a) * alpha,
           2 * ((a - 1) - (a + 1) * np.cos(w0)),
           (a + 1) - (a - 1) * np.cos(w0) - 2 * np.sqrt(a) * alpha]
    return np.array(b) / den[0], np.array(den) / den[0]


def _highpass(rate, f0=38.13547087602444, q=0.5003270373238773):
    w0 = 2 * np.pi * f0 / rate
    alpha = np.sin(w0) / (2 * q)
    b = [(1 + np.cos(w0)) / 2, -(1 + np.cos(w0)), (1 + np.cos(w0)) / 2]
    den = [1 + alpha, -2 * np.cos(w0), 1 - alpha]
    return np.array(b) / den[0], np.array(den) / den[0]


def kweight(x, rate=RATE):
    """The K-weighting curve, applied in the frequency domain (padded, so short files are fine)."""
    n = len(x)
    size = 1 << int(np.ceil(np.log2(n + rate)))
    z = np.exp(-1j * 2 * np.pi * np.fft.rfftfreq(size))
    h = np.ones_like(z)
    for b, a in (_shelf(rate), _highpass(rate)):
        h *= (b[0] + b[1] * z + b[2] * z ** 2) / (a[0] + a[1] * z + a[2] * z ** 2)
    out = np.empty_like(x)
    for c in range(x.shape[1]):
        out[:, c] = np.fft.irfft(np.fft.rfft(x[:, c], size) * h, size)[:n]
    return out


def window_levels(x, seconds, rate=RATE):
    """K-weighted level of every window of `seconds` (hop a quarter of it), LUFS-like."""
    p = (kweight(x, rate) ** 2).sum(1)
    n = int(seconds * rate)
    if len(p) < n:
        p = np.pad(p, (0, n - len(p)))
    c = np.concatenate([[0], np.cumsum(p)])
    at = np.arange(0, len(p) - n + 1, max(1, n // 4))
    return -0.691 + 10 * np.log10(np.maximum((c[at + n] - c[at]) / n, 1e-12))


def loudest(x, seconds=0.1, rate=RATE):
    """The loudest window of `seconds`, K-weighted: how loud a short sound is heard at its peak."""
    return float(window_levels(x, seconds, rate).max())


# --- spectra --------------------------------------------------------------------------

def bandpass(x, lo, hi, rate=RATE):
    """A mono brick-wall band-pass of a stereo signal."""
    m = x.mean(1)
    spectrum = np.fft.rfft(m)
    f = np.fft.rfftfreq(len(m), 1 / rate)
    spectrum[(f < lo) | (f > hi)] = 0
    return np.fft.irfft(spectrum, len(m))


def short_levels(mono, seconds=0.005, rate=RATE):
    """Level of every window of `seconds` of a mono signal, in dB."""
    n = int(seconds * rate)
    p = mono ** 2
    if len(p) < n:
        p = np.pad(p, (0, n - len(p)))
    c = np.concatenate([[0], np.cumsum(p)])
    at = np.arange(0, len(p) - n + 1, max(1, n // 4))
    return 10 * np.log10(np.maximum((c[at + n] - c[at]) / n, 1e-18))


def band_frames(x, centres, window=1024, hop=256, rate=RATE):
    """Level per third-octave band and per frame, in dB: an array [band][frame]."""
    m = x.mean(1)
    if len(m) < window:
        m = np.pad(m, (0, window - len(m)))
    w = np.hanning(window)
    f = np.fft.rfftfreq(window, 1 / rate)
    power = np.array([np.abs(np.fft.rfft(m[i:i + window] * w)) ** 2
                      for i in range(0, len(m) - window + 1, hop)]) * (4 / w.sum() ** 2)
    rows = []
    for c in centres:
        inside = (f >= c / 2 ** (1 / 6)) & (f < c * 2 ** (1 / 6))
        rows.append(10 * np.log10(np.maximum(power[:, inside].sum(1), 1e-18)))
    return np.array(rows)


# --- pictures -------------------------------------------------------------------------

def write_png(path, rgb):
    """Writes an (h, w, 3) uint8 array as a PNG, without any imaging library."""
    h, w, _ = rgb.shape
    raw = b"".join(b"\0" + rgb[y].tobytes() for y in range(h))

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    with open(str(path), "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def spectrogram(x, seconds, rate=RATE, width=640, height=320, low=50.0, high=16000.0):
    """A picture of `seconds` of sound: time left to right, frequency low to high on a log
    scale, level as colour over 80 dB (black, red, yellow, white). Blue lines mark 100 Hz,
    1 kHz and 10 kHz; grey ticks mark time, every 50 ms up to half a second of sound, every
    100 ms up to a second, every 500 ms beyond."""
    n = int(seconds * rate)
    m = np.pad(x.mean(1), (0, max(0, n - len(x))))[:n]
    window = 1024 if seconds <= 1.0 else 2048
    padded = np.pad(m, (window // 2, window // 2))
    w = np.hanning(window)
    starts = np.linspace(0, n - 1, width).astype(int)
    mag = np.array([np.abs(np.fft.rfft(padded[s:s + window] * w)) for s in starts]).T / (w.sum() / 2)
    f = np.fft.rfftfreq(window, 1 / rate)
    rows = np.exp(np.linspace(np.log(high), np.log(low), height))
    image = np.empty((height, width))
    for r, centre in enumerate(rows):
        inside = (f >= centre / 2 ** (1 / 24)) & (f <= centre * 2 ** (1 / 24))
        image[r] = mag[inside].max(0) if inside.any() else mag[np.argmin(np.abs(f - centre))]
    v = np.clip((20 * np.log10(np.maximum(image, 1e-6)) + 80) / 80, 0, 1)
    rgb = np.stack([np.clip(v * 3, 0, 1), np.clip(v * 3 - 1, 0, 1), np.clip(v * 3 - 2, 0, 1)], -1)
    for mark in (100, 1000, 10000):
        rgb[int(np.argmin(np.abs(rows - mark))), ::4] = [0.2, 0.6, 0.9]
    step = 0.05 if seconds <= 0.5 else (0.1 if seconds <= 1.0 else 0.5)
    for t in np.arange(step, seconds, step):
        rgb[::6, int(t / seconds * (width - 1))] = [0.35, 0.35, 0.35]
    return (rgb * 255).astype(np.uint8)
