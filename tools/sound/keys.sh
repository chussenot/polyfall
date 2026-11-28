# The three words a session is written in. record.sh sources this, then the session.
#
#   press <key>...          tap each key in turn, as a finger does: down, 60 ms, up
#   hold <key> <seconds>    keep a key down that long
#   rest <seconds>          wait
#
# Keys go by their X names: Left Right Up Down space p r, and F1 to F8 for the staging keys.
# Each one is written to the key log with the time it went down, in seconds on the recording's
# clock, which is how `bench.py take` can print keys and sounds on one timeline.

_stamp() {
    local us=$(( ${EPOCHREALTIME/[.,]/} - T0_US ))
    printf '%d.%06d' $(( us / 1000000 )) $(( us % 1000000 ))
}

press() {
    local key
    for key in "$@"; do
        xdotool keydown "$key"
        echo "$(_stamp) $key" >>"$KEYLOG"
        sleep 0.06
        xdotool keyup "$key"
    done
}

hold() {
    xdotool keydown "$1"
    echo "$(_stamp) down:$1" >>"$KEYLOG"
    sleep "$2"
    xdotool keyup "$1"
    echo "$(_stamp) up:$1" >>"$KEYLOG"
}

rest() { sleep "$1"; }
