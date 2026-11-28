# M mutes everything, the theme included, and M again brings it all back.
# seconds: 8
# needs: stage
# expect: lock+hard_drop+clear_2
#
# Two clears, one each side of a mute: the one made while muted must not be heard. The theme's
# volume in the timeline drops to nothing on the first M and comes back on the second.

rest 1.2
press m;  rest 0.3
press F1;  rest 0.15;  press space;  rest 1.5
press m;  rest 0.3
press F2;  rest 0.15;  press space;  rest 2.5
