#!/usr/bin/env bash
# Records what the game really plays while a session presses its keys.
#
#   tools/sound/record.sh <session>     one session: a name from tools/sound/sessions, or a file
#   tools/sound/record.sh --all         every session in tools/sound/sessions, then a summary
#   tools/sound/record.sh --list        the sessions there are
#
# The game runs as built, in a window on a virtual screen (Xvfb), with a virtual sound card: a
# PulseAudio server started for the take and gone after it. Keys reach the game through the X
# server, as a keyboard's do (xdotool). What the card is sent is recorded sample for sample.
# None of it touches the screen, the keyboard or the sound of the machine it runs on.
#
# The take lands in target/sound/takes/<session>/ and is then taken apart by `bench.py take`,
# whose verdict is this script's exit status. A take the virtual card dropped out of is not a
# verdict on the game: it is recorded again, up to three times.
#
# Linux only. Needs Xvfb, xdotool, PulseAudio with its command-line tools, ALSA's PulseAudio
# plugin, bash 5, python3 with numpy, and what the game itself needs to build.
# See tools/sound/README.md.

set -euo pipefail
export LC_ALL=C

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$here/../.." && pwd)
sessions="$here/sessions"
attempts=3
unusable=3      # the exit status of `bench.py take` for a recording that cannot be judged

source "$here/keys.sh"      # press, hold, rest: the words sessions are written in

die() {
    echo "record.sh: $*" >&2
    exit 2
}

# header <file> <name>: the value of the file's "# name: value" line, or nothing
header() {
    sed -n "s/^# $2: *//p" "$1" | head -n 1
}

check_tools() {
    local missing=() tool
    for tool in Xvfb xdotool pulseaudio pactl parec pacat python3 cargo timeout; do
        command -v "$tool" >/dev/null || missing+=("$tool")
    done
    if [ ${#missing[@]} -gt 0 ]; then
        die "missing: ${missing[*]}.
On Debian or Ubuntu: apt-get install xvfb xdotool pulseaudio pulseaudio-utils libasound2-plugins"
    fi
    [ -n "${EPOCHREALTIME:-}" ] || die "needs bash 5 or later (for its clock); this is bash $BASH_VERSION"
    python3 -c "import numpy" 2>/dev/null || die "python3 has no numpy"
}

pids=()
run=""

# Stops what this take started, and only that: never a process it did not start itself.
cleanup() {
    local pid
    for pid in "${pids[@]:-}"; do
        if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || true; fi
    done
    wait 2>/dev/null || true
    pids=()
    if [ -n "$run" ]; then rm -rf "$run"; fi
    run=""
}

# record <session file> <take dir> <binary> <seconds>
record() {
    local session=$1 take=$2 bin=$3 seconds=$4
    local rec game wid status alive=yes

    rm -rf "$take"
    mkdir -p "$take"
    cp -f "$session" "$take/session.sh"
    # sockets live here, and a socket's path may not pass 107 characters: so not under the take
    run=$(mktemp -d "${TMPDIR:-/tmp}/pfs.XXXXXX")
    [ ${#run} -le 94 ] || die "TMPDIR is too long a path for a socket ($run); set TMPDIR=/tmp"

    # a screen
    Xvfb -displayfd 3 -screen 0 560x660x24 -nolisten tcp 3>"$run/display" >/dev/null 2>&1 &
    pids+=($!)
    for _ in $(seq 50); do [ -s "$run/display" ] && break; sleep 0.1; done
    [ -s "$run/display" ] || die "Xvfb did not start"
    export DISPLAY=":$(cat "$run/display")"

    # a sound card, private to this take
    export XDG_RUNTIME_DIR="$run" XDG_CONFIG_HOME="$run/config" PULSE_SERVER="unix:$run/pulse/native"
    mkdir -p "$run/config"
    pulseaudio -n --daemonize=no --exit-idle-time=-1 --log-target="file:$take/pulse.log" \
        --load=module-native-protocol-unix \
        --load="module-null-sink sink_name=bench rate=44100 channels=2 format=float32le" \
        --load=module-always-sink >/dev/null 2>&1 &
    pids+=($!)
    for _ in $(seq 50); do pactl info >/dev/null 2>&1 && break; sleep 0.1; done
    pactl info >/dev/null 2>&1 || die "PulseAudio did not start; see $take/pulse.log"
    pactl set-default-sink bench

    # the recorder, on what the card is sent
    parec --device=bench.monitor --format=float32le --rate=44100 --channels=2 --latency-msec=20 \
        >"$take/out.raw" 2>>"$take/pulse.err" &
    rec=$!
    pids+=("$rec")
    for _ in $(seq 50); do [ -s "$take/out.raw" ] && break; sleep 0.1; done
    [ -s "$take/out.raw" ] || die "the recorder gets nothing from the virtual card; see $take/pulse.err"
    sleep 0.3

    # one clock for keys and sound: a click goes into the card at a known time
    local t0=$EPOCHREALTIME
    export T0_US=${t0/[.,]/} KEYLOG="$take/keys"
    : >"$KEYLOG"
    python3 "$here/marker.py" "$t0" 0.25 "$KEYLOG" |
        pacat --playback --device=bench --format=s16le --rate=44100 --channels=2 --latency-msec=10 \
            2>>"$take/pulse.err" &
    sleep 0.2

    # the game. A latency of 150 ms is what keeps the virtual card from dropping out on a busy machine.
    ALSA_CONFIG_PATH="$here/asound.conf" PULSE_LATENCY_MSEC=150 "$bin" >"$take/game.log" 2>&1 &
    game=$!
    pids+=("$game")
    wid=$(timeout 20 xdotool search --sync --onlyvisible --limit 1 --name '^Polyfall$') ||
        die "the game's window did not appear; see $take/game.log"
    xdotool windowfocus "$wid" 2>/dev/null || true
    xdotool mousemove 280 330
    sleep 1.0

    # the session
    set +e
    (
        set -e
        source "$session"
    )
    status=$?
    set -e
    [ $status -eq 0 ] || die "the session stopped on an error (status $status)"

    while (( ${EPOCHREALTIME/[.,]/} - T0_US < seconds * 1000000 )); do sleep 0.1; done
    kill -0 "$game" 2>/dev/null || alive=no
    echo "$(_stamp) END" >>"$KEYLOG"
    kill "$game" 2>/dev/null || true
    sleep 0.3
    kill "$rec" 2>/dev/null || true
    wait "$rec" 2>/dev/null || true
    cleanup

    # floats, as the card got them: a sum over full scale is kept, not clipped
    python3 -c 'import sys; sys.path.insert(0, sys.argv[1]); import numpy, sig
sig.write_wav(sys.argv[3], numpy.fromfile(sys.argv[2], "<f4").reshape(-1, 2), floats=True)' \
        "$here" "$take/out.raw" "$take/out.wav"
    rm -f "$take/out.raw"
    [ $alive = yes ] || die "the game stopped before the end of the session; see $take/game.log"
    if grep -qE "ALSA lib|Audio thread died" "$take/game.log"; then
        die "the game could not open the virtual sound card. Is ALSA's PulseAudio plugin installed
(Debian and Ubuntu: libasound2-plugins)? See $take/game.log"
    fi
}

one() {
    local session=$1 name seconds bin target take status try
    [ -f "$session" ] || session="$sessions/$1.sh"
    [ -f "$session" ] || die "no session called $1; --list shows the ones there are"
    name=$(basename "$session" .sh)
    seconds=$(header "$session" seconds)
    [[ $seconds =~ ^[0-9]+$ ]] || die "$session: needs a '# seconds: N' line, how long to record"

    check_tools
    target=${CARGO_TARGET_DIR:-$root/target}
    if [ "$(header "$session" needs)" = stage ]; then
        # its own build directory: target/release/polyfall stays the game as shipped
        cargo build --quiet --release --features stage --manifest-path "$root/Cargo.toml" --target-dir "$target/stage"
        bin="$target/stage/release/polyfall"
    else
        cargo build --quiet --release --manifest-path "$root/Cargo.toml"
        bin="$target/release/polyfall"
    fi
    take="$root/target/sound/takes/$name"

    for try in $(seq $attempts); do
        echo "recording $name: $seconds s"
        record "$session" "$take" "$bin" "$seconds"
        status=0
        python3 "$here/bench.py" take "$name" || status=$?
        if [ $status -ne $unusable ] || [ "$try" -eq $attempts ]; then break; fi
        echo "recording it again ($((try + 1)) of $attempts)"
    done
    if [ $status -eq 0 ] && [ "$(header "$session" then)" = seam ]; then
        python3 "$here/bench.py" seam "$name" || status=$?
    fi
    return $status
}

trap cleanup EXIT

case "${1:-}" in
"" | -h | --help)
    sed -n '2,/^$/s/^# \{0,1\}//p' "$0"
    exit 2
    ;;
--list)
    for f in "$sessions"/*.sh; do
        printf '%-8s %s\n' "$(basename "$f" .sh)" "$(sed -n '1s/^# *//p' "$f")"
    done
    ;;
--all)
    failed=()
    for f in "$sessions"/*.sh; do
        name=$(basename "$f" .sh)
        echo
        echo "=== $name"
        "$0" "$name" || failed+=("$name")
    done
    echo
    if [ ${#failed[@]} -gt 0 ]; then
        echo "sessions with problems: ${failed[*]}"
        exit 1
    fi
    echo "every session: all good"
    ;;
*)
    one "$1"
    ;;
esac
