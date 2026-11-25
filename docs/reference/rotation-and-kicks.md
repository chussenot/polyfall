---
title: Rotation and kicks
description: How a piece turns, the six positions tried in order when the turned piece does not fit, and the cases in which a turn is refused.
source:
  - Game::rotate
  - Game::fits
  - Piece
---

# Rotation and kicks

This page explains what happens when the player presses Up. Read it when a
rotation surprises you, or before you change `Game::rotate`, which holds all of
this logic.

## The idea

A rotation has two parts. First the piece is turned a quarter turn. Then the
game looks for a place where the turned piece fits: its current position first,
then a few positions next to it. Such a shift is a kick. It lets a piece turn
when it touches a wall or the stack. If no position fits, the piece stays as it
was.

## The turn

Each cell offset `(x, y)` of the piece becomes `(-y, x)`:

```rust
let turned: Vec<_> = self.piece.cells.iter().map(|&(x, y)| (-y, x)).collect();
```

Rows are numbered downward, so the player sees a quarter turn clockwise: the
cell right of the centre, `(1, 0)`, goes below it, `(0, 1)`.

- The piece turns around its cell `(0, 0)`. That is the first cell
  `random_piece` created, not the middle of the shape.
- There is one direction of rotation. No key turns the other way.
- Four turns in open space bring the piece back to the same cells at the same
  position.

## The kicks

`rotate` tests the turned piece with `Game::fits` at the piece position plus
each offset below, in this order. It applies the first one that fits: the piece
moves by the offset and takes the turned cells.

| Order | Offset `(dx, dy)` | Meaning | Tried when |
| --- | --- | --- | --- |
| 1 | `(0, 0)` | In place | Always |
| 2 | `(-1, 0)` | One column left | Always |
| 3 | `(1, 0)` | One column right | Always |
| 4 | `(-2, 0)` | Two columns left | Only if `wall_only(-1)` is true |
| 5 | `(2, 0)` | Two columns right | Only if `wall_only(1)` is true |
| 6 | `(0, 1)` | One row down | Always |

Kick 6 exists for a piece at the top of the well: its turned shape may reach
above row 0, and one row down makes room.

### When a two-column kick is allowed

A two-column kick may only move a piece away from a side wall. It must never
carry a piece over locked cells.

Before it tries kick 4 or 5, `rotate` calls the closure `wall_only` with the
direction of the kick: `-1` for left, `1` for right. `wall_only` looks at the
turned piece moved one column in that direction, which is the position kick 2
or kick 3 tried. It is true when both of these hold there:

- at least one cell is outside the columns `0..W`;
- no cell is on a locked cell.

In other words, the side wall alone stopped the one-column kick. Only then is
the two-column position tested, and it must still pass `fits`.

### Three examples

All three come from the tests. A position is written `(x, y)`.

| Start | Result | Why |
| --- | --- | --- |
| Cells `(0,0)`, `(0,1)` at `(5, 10)`, with locked cells in columns 4 and 6 on rows 9 to 12 | No rotation | Kicks 1 to 3 hit locked cells. Kicks 4 and 5 are not allowed, because no wall is involved. Kick 6 hits a locked cell. |
| Cells `(0,-1)`, `(0,0)`, `(0,1)`, `(0,2)` at `(0, 10)`, against the left wall | Cells `(1,0)`, `(0,0)`, `(-1,0)`, `(-2,0)` at `(2, 10)` | Kicks 1 to 4 leave a cell outside the left wall. Kick 5 is allowed, because only the wall stopped kick 3, and it fits. |
| Cells `(0,0)`, `(-1,0)`, `(1,0)` at `(5, 0)`, on the top row | Cells `(0,0)`, `(0,-1)`, `(0,1)` at `(5, 1)` | The turned piece reaches above row 0. Kick 6 fits. |

The tests are `rotation_kicks_off_walls_but_not_over_the_stack` (first two rows)
and `top_row_piece_can_still_turn`.

## When a turn is refused

- **Two rows above the well.** Only one row down is tried. A turned piece that
  would reach two or more rows above row 0 does not turn. The code marks this
  as a known shortcut: `shortcut: a single row down, so a shape poking 2+ rows
  above row 0 still refuses; add more rows if that matters`.
- **No kick moves the piece up.** A turned piece that would go below the floor
  or into the stack beneath it turns only if a sideways kick fits.
- **Kicks are not combined.** The row down is tried last and never together
  with a sideways kick.
- **No feedback.** A refused turn draws nothing and records nothing.
