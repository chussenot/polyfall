# A soft drop all the way down: the run of ticks, then a lock that clears a row.
# seconds: 6
# needs: stage
# expect-within: soft_drop | lock+clear_1
#
# Down stays held past the lock, so the next piece is soft-dropped too and the run starts again;
# hence "within". How long the fall takes depends on the frame rate.

rest 1.2
press F1;  rest 0.15
hold Down 1.4
rest 2.0
