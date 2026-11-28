# One to five rows cleared by a hard drop; the four-row clear also raises the level.
# seconds: 19
# needs: stage
# expect: lock+hard_drop+clear_1 | lock+hard_drop+clear_2 | lock+hard_drop+clear_3 | lock+hard_drop+clear_4+level_up | lock+hard_drop+clear_5
#
# The loudest things the game does, each from the longest fall. The level-up lands on the
# four-row clear because that is the stack the mix has least room for.

rest 1.2
press F1;  rest 0.15;  press space;  rest 2.0
press F2;  rest 0.15;  press space;  rest 2.0
press F3;  rest 0.15;  press space;  rest 2.2
press F6 F4;  rest 0.15;  press space;  rest 5.0
press F5;  rest 0.15;  press space;  rest 3.5
