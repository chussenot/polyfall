---
title: Rendering and layout
description: The fixed 560 by 660 view, the order in which a frame is drawn, and the position, size and colour of everything in the well, the side panel and the overlays.
source:
  - CELL
  - PANEL
  - VIEW_W
  - VIEW_H
  - TEXT
  - DIM
  - BG
  - WELL
  - COLORS
  - GUIDE
  - LANDING_ALPHA
  - GLOW
  - GLOW_FADE
  - conf
  - Game::draw
  - Game::draw_well
  - Game::draw_panel
  - Game::draw_overlay
  - Game::landing_y
  - draw_piece
---

# Rendering and layout

This page describes what is drawn, where and in what colour. Read it before you
move, resize or recolour anything on screen.

## The idea

The window is split in two: the well on the left, 360 units wide, and the side
panel on the right, 200 units wide. Each frame is drawn from nothing, back to
front: background, well, side panel, then overlays on top of the well.
Everything is a rectangle or a line of text. There are no images and no custom
fonts.

This page gives the resting picture: what is on screen when nothing is moving.
[Effects](effects.md) describes what is added for a short time after an event.

## The fixed view

All positions on this page are in one coordinate system: 560 units wide and 660
high, with the origin at the top left and y growing downward.

| Constant | Definition | Value | Meaning |
| --- | --- | --- | --- |
| `CELL` | `30.0` | 30 | Side of one cell of the well. |
| `PANEL` | `200.0` | 200 | Width of the side panel. |
| `VIEW_W` | `W as f32 * CELL + PANEL` | 560 | Width of the view. |
| `VIEW_H` | `H as f32 * CELL` | 660 | Height of the view. |

`conf()` asks for a window of that size that cannot be resized. Every frame,
`Game::draw` also sets a camera that maps this 560 by 660 rectangle onto the
whole window:

```rust
set_camera(&Camera2D::from_display_rect(Rect::new(
    0.0, VIEW_H, VIEW_W, -VIEW_H,
)));
```

The comment in the code gives the reason: "fixed logical view: miniquad shrinks
the logical screen on scaled displays, which cropped the layout".

## Order of a frame

`Game::draw` runs once per frame, in every game state. It reads the `Game` and
the `Fx` values and changes neither.

1. Fill the window with `BG`.
2. Set the camera, moved down by `Fx::kick_px()`: an impact knocks the well a
   few units down. At rest this is `0`.
3. `draw_well`: the well, the stack and the falling piece.
4. Set the camera again, not moved. The side panel and the overlays never
   shake.
5. `draw_panel`: the side panel.
6. `draw_overlay`: the pause or game-over overlay.

## The well

`draw_well` covers x from 0 to 360 and the full height. It draws, in order:

1. A `WELL` rectangle over the whole well.
2. A `GUIDE` tint over every second column: columns 1, 3, 5, 7, 9 and 11.
3. `Fx::draw_under`: light on the floor of the well, under the blocks.
4. Every locked cell.
5. `Fx::draw_clear`: rows that were just removed, while they break up.
6. The landing outline.
7. The falling piece.
8. `Fx::draw_over`: contact lines, the hard-drop ring and fragments.

A cell is a filled square of 28 units (`CELL - 2`), inset by 1 unit on each
side. Two neighbouring cells are therefore 2 units apart.

`Fx::block(x, y)` returns the rectangle of each locked cell. At rest it is the
square above. For a short time after a lock it is compressed, and after a row
is removed it is higher than its real place.

`draw_piece` draws the falling piece. `Fx::piece_pose()` gives it an offset and
an angle, both zero at rest, so that a move or a turn is seen as a short
motion.

### Landing outline

`Game::landing_y()` moves the falling piece straight down for as long as it
fits and returns the row it reaches. Each cell of the piece is outlined at that
position: a square of 26 units (`CELL - 4`), inset by 2, with a line 2 units
thick, in the colour of the piece at alpha `LANDING_ALPHA` (`0.5`).

The falling piece is drawn after the outline and covers it where they overlap.
No outline is drawn when the piece is already on its landing row.

The outline always uses the real position of the piece, not the drawn one. It
moves in the same frame as the key.

## The side panel

`draw_panel` draws right of the well. Its content starts at x = 380
(`W as f32 * CELL + 20.0`). For text, y is the baseline.

| Element | x | y | Size | Colour |
| --- | --- | --- | --- | --- |
| Label `SCORE` | 380 | 34 | 16 | `DIM` |
| Score, as given by `Fx::score` | 380 | 74 | 48 | `TEXT`, plus `Fx::score_tint` |
| Gain `+<points>`, for 1 s after scoring | right edge at 540 | 34 | 16 | Colour of the piece that scored |
| Label `LINES` | 380 | 112 | 16 | `DIM` |
| Rows removed (`lines`) | 380 | 140 | 28 | `TEXT` |
| Label `LEVEL` | 460 | 112 | 16 | `DIM` |
| Level (`level() + 1`) | 460 | 140 | 28 | `TEXT`, plus the [level highlight](#level-highlight) |
| Label `NEXT` | 380 | 182 | 16 | `DIM` |
| Slot for `next` | 380 | 192 | 160 by 160 | See below |
| Label `THEN` | 380 | 392 | 14 | `DIM` |
| Slot for `after` | 380 | 402 | 80 by 80 | See below |
| Key list, 5 rows | 380 and 476 | 522, then 24 more per row | 18 | Key in `TEXT`, action in `DIM` |

### Upcoming pieces

Each of the two upcoming pieces is drawn in a square slot: a `WELL` square with
a `DIM` outline 2 units thick. The piece is centred in its slot in both
directions.

| Piece | Label | Slot side | Cell size |
| --- | --- | --- | --- |
| `next` | `NEXT` | 160 | 30 (`CELL`) |
| `after` | `THEN` | 80 | 15 (`CELL / 2.0`) |

A piece is at most 5 cells wide or high, which is 150 or 75 units, so it always
fits its slot.

### Key list

Five rows, in this order: `Left/Right` `move`, `Up` `rotate`, `Down`
`soft drop`, `Space` `hard drop`, `P` `pause`. R is not in the list. It appears
only in the overlays.

## Colours

All colours are constants built with `Color::new(r, g, b, a)`.

| Constant | r, g, b, a | Used for |
| --- | --- | --- |
| `BG` | 0.07, 0.08, 0.11, 1.0 | Window background and overlays. |
| `WELL` | 0.12, 0.13, 0.18, 1.0 | Background of the well and of the slots. |
| `GUIDE` | 1.0, 1.0, 1.0, 0.025 | Tint on every second column. |
| `TEXT` | 0.90, 0.91, 0.94, 1.0 | Numbers, key names, main overlay lines. |
| `DIM` | 0.50, 0.53, 0.62, 1.0 | Labels, action names, outlines. |
| `COLORS[0]` | 0.36, 0.52, 0.85, 1.0 | Pieces of 3 cells (blue). |
| `COLORS[1]` | 0.88, 0.58, 0.36, 1.0 | Pieces of 4 cells (orange), and the level highlight. |
| `COLORS[2]` | 0.92, 0.88, 0.70, 1.0 | Pieces of 5 cells (pale sand). |

Effects add no colour of their own. They use the colour of the piece or cell
concerned, `COLORS[1]` for the level, and white for light.

The colour of a piece tells its cell count, not its shape. The comment on
`COLORS` says the three colours rise in lightness "so they also read without
hue". `Piece.color` and the
values in the well are 1, 2 or 3; the colour drawn is `COLORS[value - 1]`. A
locked cell keeps the colour of its piece.

## Overlays

`draw_overlay` runs last and draws only over the well.

### Pause and game-over overlay

It is drawn when the game is over or paused.

| State | Line 1 | Line 2 | Line 3 |
| --- | --- | --- | --- |
| Over | `Game over` | `Score <score>` | `R  new game` |
| Paused | `Paused` | `P  resume` | `R  new game` |

- `BG` at alpha `0.7` dims the well, drawn as one rectangle per row. The side
  panel is not dimmed.
- A box of 260 by 150 is centred in the well, with its top left corner at
  (50, 255). It is filled with `BG` and outlined in `DIM`, 2 units thick.
- Each line is centred in the well with `measure_text`.
- A pause shows all of this at once. At the end of a game the final state is
  held, the rows dim from the top down and the box fades in. `Fx::over_dim`
  and `Fx::over_box` give the progress. See
  [Effects](effects.md#game-over).

| Line | y below the top of the box | Size | Colour |
| --- | --- | --- | --- |
| 1 | 52 | 40 | `TEXT` |
| 2 | 92 | 24 | `TEXT` |
| 3 | 124 | 20 | `DIM` |

## Level highlight

The level number is drawn a second time, over the first, in `COLORS[1]`. Its
alpha is `((GLOW - t) / GLOW_FADE)` kept between 0 and 1, where `t` is
`Fx::levelled()`, the seconds since the level last rose. With `GLOW` = `1.3` s
and `GLOW_FADE` = `0.7` s, the number is fully orange for 0.6 s, then fades
back to `TEXT` over 0.7 s.

The rest of the level change, and every other effect, is on
[Effects](effects.md).
