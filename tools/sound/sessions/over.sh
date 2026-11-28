# The lock that ends the game, the ending, and a new game started while it still plays.
# seconds: 10
# needs: stage
# expect: lock+hard_drop+game_over | new_game | move_left | lock+hard_drop
#
# The ending replaces the theme; the new game cuts the ending and starts the theme from its top.

rest 1.2
press F7;  rest 0.15;  press space;  rest 3.0
press r;  rest 1.5
press Left;  rest 0.4;  press space;  rest 1.5
