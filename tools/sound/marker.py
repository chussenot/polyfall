"""Puts one click into the virtual sound card and writes down when, so that a recording and the
key log can be read on one clock.

usage: marker.py <t0> <click after seconds> <key log>      raw 16-bit stereo, 44.1 kHz, on stdout

t0 is the clock's zero, in seconds since the epoch. The click is three milliseconds of 4 kHz
near full scale: nothing the game plays that early is as loud, so the bench finds it by level
and then blanks it.

A player given one short click on its own may sit on it until its buffer fills. So this feeds
it silence all along, a little slower than real time: the player's buffer stays empty and the
click goes out when it is written.
"""
import math
import struct
import sys
import time

RATE = 44100
t0, click_at, log = float(sys.argv[1]), float(sys.argv[2]), sys.argv[3]
wave = (int(0.98 * 32767 * math.sin(2 * math.pi * 4000 * i / RATE)) for i in range(RATE * 3 // 1000))
click = b"".join(struct.pack("<hh", v, v) for v in wave)
silence = bytes(RATE // 100 * 4)        # 10 ms
out = sys.stdout.buffer
clicked = False
try:
    while time.time() - t0 < click_at + 1.0:
        if not clicked and time.time() - t0 >= click_at:
            out.write(click)
            out.flush()
            with open(log, "a") as f:
                f.write("%.6f MARKER\n" % (time.time() - t0))
            clicked = True
        out.write(silence)
        out.flush()
        time.sleep(0.0102)
except BrokenPipeError:
    pass
