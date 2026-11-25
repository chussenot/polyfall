---
title: Rendering and layout
description: The fixed 560 x 660 logical view and its reason, drawing order, the well, the side panel, colours by piece size, overlays and feedback timers.
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

Everything is drawn with macroquad rectangles and text. There are no images,
textures or custom fonts.

## The fixed logical view

| Constant | Definition | Value |
| --- | --- | --- |
| `CELL` | `30.0` | Side of one well cell. |
| `PANEL` | `200.0` | Width of the side panel. |
| `VIEW_W` | `W as f32 * CELL + PANEL` | 560 |
| `VIEW_H` | `H as f32 * CELL` | 660 |

`conf()` asks for a 560 x 660 window that is not resizable, with
`high_dpi: true`. Every frame, `Game::draw` then sets a camera that maps the
rectangle from `(0, 0)` to `(560, 660)` onto the window:

```rust
set_camera(&Camera2D::from_display_rect(Rect::new(0.0, VIEW_H, VIEW_W, -VIEW_H)));
```

The reason is given by the comment above that line: "fixed logical view:
miniquad shrinks the logical screen on scaled displays, which cropped the
layout". All coordinates on this page are in this logical view, with the origin
at the top left and y growing downward.

## Drawing order

`Game::draw` runs once per frame, in every game state:

1. Decrease `flash` and `glow` by the frame time, not below `0.0`.
2. Set the camera.
3. `clear_background(BG)`.
4. `draw_well`
5. `draw_panel`
6. `draw_overlay`

`draw` takes `&mut self` only for step 1.

## The well

`draw_well` covers `x` 0 to 360, `y` 0 to 660. In order:

1. A `WELL` rectangle over the whole well.
2. A `GUIDE` tint over every odd column (1, 3, 5, 7, 9, 11), full height.
3. Every locked cell, with `draw_cell`.
4. The landing outline of the falling piece.
5. The falling piece, with `draw_cell`.

`draw_cell(x, y, color)` draws a filled square of `CELL - 2` (28) inset by 1 on
each side, in `COLORS[color - 1]`. The 2-unit gap between neighbours shows the
well background.

### Landing outline

`Game::landing_y()` returns the row the piece would reach by moving straight
down until `fits` fails. When that row differs from the current row, each cell
of the piece is outlined at the landing position: a square of `CELL - 4` (26)
inset by 2, line thickness 2, in the piece colour with alpha `LANDING_ALPHA`
(`0.5`). The piece itself is drawn afterwards and covers the outline where they
overlap. When the piece is already on its landing row, no outline is drawn.

## The panel

`draw_panel` draws to the right of the well. Its left edge for content is
`px = W as f32 * CELL + 20.0`, that is 380. The `x` and `y` values below are the
ones passed to `draw_text` or `draw_rectangle`.

| Element | x | y | Size | Colour |
| --- | --- | --- | --- | --- |
| Label `SCORE` | 380 | 34 | 16 | `DIM` |
| Score value | 380 | 74 | 48 | `TEXT` |
| Label `LINES` | 380 | 112 | 16 | `DIM` |
| Lines value | 380 | 140 | 28 | `TEXT` |
| Label `LEVEL` | 460 | 112 | 16 | `DIM` |
| Level value (`level() + 1`) | 460 | 140 | 28 | `TEXT`, plus highlight |
| Label `NEXT` | 380 | 182 | 16 | `DIM` |
| Slot for `next` | 380 | 192 | 160 x 160 | see below |
| Label `THEN` | 380 | 392 | 14 | `DIM` |
| Slot for `after` | 380 | 402 | 80 x 80 | see below |
| Key list, 5 rows | 380 and 476 | 522 + 24 per row | 18 | key `TEXT`, action `DIM` |

### Preview slots

Both upcoming pieces use the same closure, `preview(piece, by, side, cell)`:

- a `WELL` square of `side`, with a `DIM` outline of thickness 2;
- the piece drawn with cells of size `cell`, each a filled square of `cell - 2`
  inset by 1, in the piece colour;
- the bounding box of the piece centred in the slot on both axes.

| Piece | Slot side | Cell size |
| --- | --- | --- |
| `next` | 160 | `CELL` = 30 |
| `after` | 80 | `CELL / 2.0` = 15 |

A piece spans at most 5 cells in either direction, which is 150 or 75 units, so
it always fits its slot.

### Key list

Five fixed rows, drawn in this order:

| Key text | Action text |
| --- | --- |
| `Left/Right` | `move` |
| `Up` | `rotate` |
| `Down` | `soft drop` |
| `Space` | `hard drop` |
| `P` | `pause` |

R is not in this list. It appears only in the pause and game-over overlays.

## Colours

All colours are `Color::new(r, g, b, a)` constants.

| Constant | r, g, b, a | Used for |
| --- | --- | --- |
| `BG` | 0.07, 0.08, 0.11, 1.0 | Window background, overlay dimming and box. |
| `WELL` | 0.12, 0.13, 0.18, 1.0 | Well background, preview slots. |
| `GUIDE` | 1.0, 1.0, 1.0, 0.025 | Tint on every other column. |
| `TEXT` | 0.90, 0.91, 0.94, 1.0 | Values, key names, main overlay lines. |
| `DIM` | 0.50, 0.53, 0.62, 1.0 | Labels, action names, outlines. |
| `COLORS[0]` | 0.36, 0.52, 0.85, 1.0 | Pieces of 3 cells (grid value 1). |
| `COLORS[1]` | 0.88, 0.58, 0.36, 1.0 | Pieces of 4 cells (grid value 2); level highlight. |
| `COLORS[2]` | 0.92, 0.88, 0.70, 1.0 | Pieces of 5 cells (grid value 3). |

The comment on `COLORS` describes them as "blue / orange / pale sand, rising in
lightness so they also read without hue". A piece's colour depends only on its
cell count, never on its shape, and locked cells keep the colour of the piece
they came from.

## Overlays

`draw_overlay` is drawn last and only over the well (`x` 0 to 360).

### Cleared-row band

For each row index in `cleared`, a white rectangle one cell high and as wide as
the well is drawn with alpha `0.35 * flash / FLASH`. `flash` starts at `FLASH`
(`0.25` s) when a lock clears rows and falls to zero, so the band fades from
alpha 0.35 to 0 in a quarter of a second.

`cleared` holds the row indices as they were before the stack moved down. The
band is drawn at those rows after the collapse, over whatever is there now.

### Pause and game-over box

Drawn when `over` or `paused` is true; `over` is tested first.

| State | Line 1 | Line 2 | Line 3 |
| --- | --- | --- | --- |
| Over | `Game over` | `Score <score>` | `R  new game` |
| Paused | `Paused` | `P  resume` | `R  new game` |

- The well is covered by a `BG` rectangle with alpha `0.7`. The side panel is
  not dimmed.
- A box of 260 x 150 is centred in the well, at `(50, 255)`, filled with `BG`
  and outlined in `DIM` with thickness 2.
- The three lines are centred horizontally in the well using `measure_text`,
  at `y` offsets 52, 92 and 124 from the top of the box, with sizes 40, 24 and
  20, in `TEXT`, `TEXT` and `DIM`.

## Feedback timers

| Field | Set in `lock` when | Start value | Effect |
| --- | --- | --- | --- |
| `flash` | at least one row was cleared | `FLASH` = 0.25 s | Alpha of the cleared-row band. |
| `glow` | `level()` rose | `GLOW` = 1.3 s | Alpha of the level highlight. |

The level value is drawn twice at the same position: once in `TEXT`, then in
`COLORS[1]` with alpha `(glow / GLOW_FADE).min(1.0)`. With `GLOW = 1.3` and
`GLOW_FADE = 0.7`, the highlight is at full strength for the first 0.6 s and
fades linearly to zero over the last 0.7 s.

Both timers are decreased in `draw`, which runs in every state, so they keep
running during a pause and after the game is over.
