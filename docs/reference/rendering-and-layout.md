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
  - FLASH
  - GLOW
  - GLOW_FADE
  - conf
  - Game::draw
  - Game::draw_well
  - Game::draw_panel
  - Game::draw_overlay
  - Game::landing_y
  - draw_cell
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

`Game::draw` runs once per frame, in every game state:

1. Count down the two [display timers](#display-timers).
2. Set the camera.
3. Fill the window with `BG`.
4. `draw_well`: the well, the stack and the falling piece.
5. `draw_panel`: the side panel.
6. `draw_overlay`: the band over removed rows, then the pause or game-over
   overlay.

## The well

`draw_well` covers x from 0 to 360 and the full height. It draws, in order:

1. A `WELL` rectangle over the whole well.
2. A `GUIDE` tint over every second column: columns 1, 3, 5, 7, 9 and 11.
3. Every locked cell.
4. The landing outline.
5. The falling piece.

`draw_cell` draws one cell as a filled square of 28 units (`CELL - 2`), inset by
1 unit on each side. Two neighbouring cells are therefore 2 units apart.

### Landing outline

`Game::landing_y()` moves the falling piece straight down for as long as it
fits and returns the row it reaches. Each cell of the piece is outlined at that
position: a square of 26 units (`CELL - 4`), inset by 2, with a line 2 units
thick, in the colour of the piece at alpha `LANDING_ALPHA` (`0.5`).

The falling piece is drawn after the outline and covers it where they overlap.
No outline is drawn when the piece is already on its landing row.

## The side panel

`draw_panel` draws right of the well. Its content starts at x = 380
(`W as f32 * CELL + 20.0`). For text, y is the baseline.

| Element | x | y | Size | Colour |
| --- | --- | --- | --- | --- |
| Label `SCORE` | 380 | 34 | 16 | `DIM` |
| Score | 380 | 74 | 48 | `TEXT` |
| Label `LINES` | 380 | 112 | 16 | `DIM` |
| Rows removed (`lines`) | 380 | 140 | 28 | `TEXT` |
| Label `LEVEL` | 460 | 112 | 16 | `DIM` |
| Level (`level() + 1`) | 460 | 140 | 28 | `TEXT`, plus the [level highlight](#display-timers) |
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

The colour of a piece tells its cell count, not its shape. The comment on
`COLORS` says the three colours rise in lightness "so they also read without
hue". `Piece.color` and the
values in the well are 1, 2 or 3; the colour drawn is `COLORS[value - 1]`. A
locked cell keeps the colour of its piece.

## Overlays

`draw_overlay` runs last and draws only over the well.

### Band over removed rows

When a lock removes rows, a white band flashes where they were. For each row
index in `cleared`, a white rectangle one cell high and as wide as the well is
drawn at alpha `0.35 * flash / FLASH`. The band fades from 0.35 to 0 in `FLASH`
(`0.25` s).

`cleared` holds the row indices from before the rows above moved down. The band
is drawn at those rows, over what is there now.

### Pause and game-over overlay

It is drawn when the game is over or paused.

| State | Line 1 | Line 2 | Line 3 |
| --- | --- | --- | --- |
| Over | `Game over` | `Score <score>` | `R  new game` |
| Paused | `Paused` | `P  resume` | `R  new game` |

- A `BG` rectangle at alpha `0.7` dims the well. The side panel is not dimmed.
- A box of 260 by 150 is centred in the well, with its top left corner at
  (50, 255). It is filled with `BG` and outlined in `DIM`, 2 units thick.
- Each line is centred in the well with `measure_text`.

| Line | y below the top of the box | Size | Colour |
| --- | --- | --- | --- |
| 1 | 52 | 40 | `TEXT` |
| 2 | 92 | 24 | `TEXT` |
| 3 | 124 | 20 | `DIM` |

## Display timers

Two fields of `Game` drive effects that fade with time. `Game::lock` starts
them; `Game::draw` counts them down by the frame time, never below `0.0`.

| Field | Started when | Start value | Effect |
| --- | --- | --- | --- |
| `flash` | A lock removes at least one row. | `FLASH` = `0.25` s | The band over removed rows. |
| `glow` | A lock raises the level. | `GLOW` = `1.3` s | The level highlight. |

The level highlight is the level number drawn a second time, over the first, in
`COLORS[1]` at alpha `(glow / GLOW_FADE).min(1.0)`. With `GLOW_FADE` = `0.7` s,
the number is fully orange for 0.6 s, then fades back to `TEXT` over 0.7 s.

`draw` runs in every state, so both timers keep running during a pause and
after the game is over.
