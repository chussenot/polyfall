# The theme alone, once round and past its loop point.
# seconds: 106
# needs: stage
# expect: none
# then: seam
#
# F8 empties the well every four seconds, so nothing ever lands, and counts as a key, so the
# game does not pause itself for want of one. The theme lasts 100.4 s.

rest 1.0
for _ in $(seq 26); do
    hold F8 0.1
    rest 3.9
done
