# Every key a player has, on random pieces: taps, a held key, a wall, turns, both drops, a pause.
# seconds: 13
# needs: stage
#
# The staging build is used only because it skips the title screen; no staging key is
# pressed. The pieces are random, so what each key meets is too: a turn may be refused, a drop may clear
# a row. There is no cue list to expect; read the timeline for whether each key got its answer.

rest 1.2
press Left;  rest 0.3;  press Left;  rest 0.3;  press Right;  rest 0.4    # three taps
hold Right 0.75;  rest 0.35                                             # held: it ratchets to the wall
press Right;  rest 0.4                                                  # a tap against the wall
press Up;  rest 0.3;  press Up;  rest 0.4                                # two turns
hold Down 0.55;  rest 0.5                                               # a soft drop
press space;  rest 0.8                                                  # a hard drop
press Left;  rest 0.12;  press space;  rest 0.8                          # a move, and a hard drop at once
press p;  rest 1.2;  press p;  rest 1.2                                  # pause, resume
