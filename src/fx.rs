//! Visual effects.
//!
//! The flow is one way. Game logic pushes an [`Event`] when something happens, `Fx::on` turns it
//! into timers and particles, `Fx::tick` ages them, and the renderer asks `Fx` how to draw.
//! Nothing in here is read back by game logic, so effects cannot change a rule, a score or a
//! timing. Every effect is a function of its own age in seconds, so it looks the same at any
//! frame rate.

use crate::{Event, Grid, CELL, COLORS, H, W};
use macroquad::prelude::*;
use std::f32::consts::FRAC_PI_2;

const COLS: usize = W as usize;
const ROWS: usize = H as usize;
const WELL_W: f32 = W as f32 * CELL;
const WELL_H: f32 = H as f32 * CELL;

// --- falling piece ---
const SLIDE_RATE: f32 = 38.0; // 1/s; the drawn piece closes 85% of the gap to its real cell in 50 ms
const SPIN_RATE: f32 = 46.0; // 1/s; same for the angle left after a turn
const SLIDE_MAX: f32 = 2.0; // cells; a long frame of key repeat must not leave a long streak
const TURN_FLASH: f32 = 0.09; // seconds the piece stays brightened after a turn
const SPAWN_SLIDE: f32 = -0.5; // cells; a new piece drops in from half a cell above its spawn row

// --- lock and hard drop ---
const SQUASH_IN: f32 = 0.025; // seconds to full compression
const SQUASH_OUT: f32 = 0.16; // seconds to recover
const LOCK_FLASH: f32 = 0.14;
const LOCK_END: f32 = 0.30; // the lock effect is dropped after this
const TRAIL: f32 = 0.20;
const SHOCK: f32 = 0.26;
const KICK: f32 = 0.16; // seconds the well takes to come back after an impact
const GRAVITY: f32 = 900.0; // view units / s², on fragments only

// --- row clear ---
const SWEEP_SPEED: f32 = 120.0; // cells / s; wall to wall in 0.1 s
const ROW_LAG: f32 = 0.012; // seconds between the sweeps of two rows cleared together
const HOLD: f32 = 0.05; // seconds a cell stays lit before it breaks up
const BURST: f32 = 0.07; // seconds a lit cell takes to shrink away
const COLLAPSE_AT: f32 = 0.17; // seconds before the rows above start to come down
const COLLAPSE_LAG: f32 = 0.008; // extra wait per row of distance from the cleared rows
const COLLAPSE_LAG_ROWS: u8 = 8;
const COLLAPSE: f32 = 0.15; // seconds one row takes to come down
const HURRY: f32 = 4.0; // how many times faster a clear plays out once another piece has locked
const INHERIT: f32 = 0.06; // seconds a new clear takes to absorb what an unfinished one left
const RIPPLE: f32 = 0.5;
const CLEAR_END: f32 = 0.6;
const MAX_SPARKS: usize = 320; // five rows at five fragments a cell is 300

// --- side panel ---
const ROLL: f32 = 0.45; // seconds the score takes to count up
const GAIN: f32 = 1.0; // seconds the "+points" label is shown
pub const LEVEL_SLIDE: f32 = 0.22; // seconds the old level number takes to give way to the new one
const LEVEL_SWEEP: f32 = 0.6; // seconds the level band takes to cross the well

// --- game over ---
const OVER_FREEZE: f32 = 0.45; // seconds the final state is held untouched
const OVER_ROW_LAG: f32 = 0.012; // the dimming runs down the well, this much later per row
const OVER_DIM: f32 = 0.4;
const OVER_BOX_AT: f32 = OVER_FREEZE + 0.35;
const OVER_BOX: f32 = 0.25;

const NEVER: f32 = 1e9; // age of an effect that has not happened

/// Fast start, soft stop. Clamps `t` to 0..1.
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Soft start, soft stop. Clamps `t` to 0..1.
fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 1 at age 0, 0 from `life` on, falling fast at first.
fn fade(age: f32, life: f32) -> f32 {
    (1.0 - (age / life).clamp(0.0, 1.0)).powi(2)
}

pub fn alpha(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// How hard a hard drop of `rows` rows lands, from 0.3 to 1: full strength from 10 rows on.
/// What is drawn and what is heard are both scaled by it.
pub fn drop_power(rows: i32) -> f32 {
    0.3 + 0.7 * (rows as f32 / 10.0).min(1.0)
}

fn piece_color(color: u8) -> Color {
    COLORS[color as usize - 1]
}

/// One square fragment. Its path is computed from its age, not integrated frame by frame.
struct Spark {
    from: Vec2,
    vel: Vec2,
    size: f32,
    life: f32,
    age: f32,
    color: u8,
}

/// The piece that has just become part of the stack.
struct Lock {
    cells: Vec<(usize, usize)>, // its cells still in the grid, where they are after any collapse
    under: Vec<(usize, usize)>, // the cells it came to rest on, same convention
    contact: Vec<(f32, f32)>,   // left end of each bottom edge that touched something, view units
    floor: f32,                 // bottom of its lowest cell; the piece compresses toward it
    carry: Vec2,                // what was left of the slide when it locked, in cells
    power: f32,                 // 0 for a piece that settled, up to 1 for a long hard drop
    age: f32,
}

impl Lock {
    /// Share of its height the piece has lost: quick compression, slower recovery.
    fn squash(&self) -> f32 {
        let amount = 0.07 + 0.13 * self.power;
        if self.age < SQUASH_IN {
            amount * self.age / SQUASH_IN
        } else {
            amount * (1.0 - ease_out((self.age - SQUASH_IN) / SQUASH_OUT))
        }
    }
}

/// What a hard drop leaves in the air: a streak per column and a ring around the landing spot.
struct Trail {
    cols: Vec<(f32, f32)>, // x of the column and top of the piece in it after landing, view units
    fell: f32,             // view units
    bounds: Rect,          // around the landed piece
    color: u8,
    power: f32,
    age: f32,
}

/// The rows a lock removed, kept on screen while they break up and the stack comes down.
struct Clear {
    rows: Vec<usize>,       // their indices before the collapse, ascending
    cells: Vec<[u8; COLS]>, // their colours
    origin: Vec<f32>,       // per row, the column the sweep starts from: where the piece closed it
    burst: Vec<u16>,        // per row, one bit per column that has thrown its fragments
    fall: [u8; ROWS],       // per row of the grid as it is now: how many rows it came down
    lag: [u8; ROWS],        // per row of the grid as it is now: its distance above the cleared rows
    age: f32,
    hurried: Option<f32>, // its age when another piece locked, if one did before it finished
    inherited: [f32; ROWS], // per row as `fall`: offset left by a clear this one replaced, <= 0
}

impl Clear {
    fn new(rows: Vec<usize>, piece: &[(i32, i32)], before: &Grid) -> Self {
        let cells = rows.iter().map(|&r| before[r]).collect();
        let origin = rows
            .iter()
            .map(|&r| {
                let xs: Vec<f32> = piece
                    .iter()
                    .filter(|c| c.1 == r as i32)
                    .map(|c| c.0 as f32 + 0.5)
                    .collect();
                // a cleared row always holds a cell of the piece; the centre is only a safe default
                if xs.is_empty() {
                    W as f32 / 2.0
                } else {
                    xs.iter().sum::<f32>() / xs.len() as f32
                }
            })
            .collect();
        let (mut fall, mut lag) = ([0; ROWS], [0; ROWS]);
        let (mut below, mut nearest) = (0, 0); // cleared rows under the row looked at; the closest
        for old in (0..ROWS).rev() {
            if rows.contains(&old) {
                (below, nearest) = (below + 1, old);
            } else if below > 0 {
                fall[old + below] = below as u8;
                lag[old + below] = ((nearest - old - 1) as u8).min(COLLAPSE_LAG_ROWS);
            }
        }
        Clear {
            burst: vec![0; rows.len()],
            rows,
            cells,
            origin,
            fall,
            lag,
            age: 0.0,
            hurried: None,
            inherited: [0.0; ROWS],
        }
    }

    /// Takes over from `old`, a clear that had not finished when this one's lock arrived, so that
    /// every row starts exactly where `old` was drawing it. `removed` are this clear's rows.
    fn inherit(&mut self, old: &Clear, removed: &[usize]) {
        for row in (0..ROWS).filter(|row| !removed.contains(row)) {
            let below = removed.iter().filter(|&&r| r > row).count();
            self.inherited[row + below] = old.row_dy(row, false);
        }
    }

    /// The time the break-up and the collapse are played at. It is the age, until another piece
    /// locks: the stack must then be where it really is almost at once, so the rest runs `HURRY`
    /// times faster and skips the wait before the collapse. Nothing jumps; it only gets quicker.
    fn clock(&self) -> f32 {
        match self.hurried {
            None => self.age,
            Some(at) => at + (self.age - at) * HURRY,
        }
    }

    /// Seconds since the sweep reached column `x` of cleared row `i`; negative before it does.
    fn reached(&self, i: usize, x: usize) -> f32 {
        // the lowest row goes first
        let wait = (self.rows.len() - 1 - i) as f32 * ROW_LAG;
        self.clock() - wait - (x as f32 + 0.5 - self.origin[i]).abs() / SWEEP_SPEED
    }

    /// How far above its real place a row of the current grid is drawn, in view units (<= 0).
    /// `fresh` is for the cells of the piece that has just locked: they are where they landed,
    /// whatever an earlier clear is still doing to the rows around them.
    fn row_dy(&self, row: usize, fresh: bool) -> f32 {
        if fresh && self.hurried.is_some() {
            return 0.0;
        }
        let wait = match self.hurried {
            None => COLLAPSE_AT,
            Some(at) => COLLAPSE_AT.min(at), // interrupted before the collapse: it starts now
        };
        let t = (self.clock() - wait - self.lag[row] as f32 * COLLAPSE_LAG) / COLLAPSE;
        let own = -(self.fall[row] as f32) * CELL * (1.0 - ease_in_out(t));
        if fresh {
            return own;
        }
        own + self.inherited[row] * (1.0 - ease_in_out(self.age / INHERIT))
    }
}

pub struct Fx {
    energy: f32,       // 0 at level 1, 1 from level 11 on; scales how lively effects are
    slide: Vec2,       // drawn piece minus real piece, in cells; decays to zero
    spin: f32,         // same for its angle, in radians
    turned: f32,       // age of the last turn
    fell: Option<i32>, // rows of the hard drop whose lock is about to arrive
    lock: Option<Lock>,
    trail: Option<Trail>,
    clear: Option<Clear>,
    sparks: Vec<Spark>,
    kick: f32, // view units the well was knocked down by
    kicked: f32,
    owed: f32, // points the shown score was behind by when the last gain arrived
    gain: u32, // points of the last scoring lock, for the "+points" label
    gain_color: u8,
    scored: f32,
    levelled: f32,
    over: Option<f32>, // age of the end of the game
    rng: u32,          // own generator: effects must not disturb the sequence of pieces
}

impl Default for Fx {
    fn default() -> Self {
        Fx {
            energy: 0.0,
            slide: Vec2::ZERO,
            spin: 0.0,
            turned: NEVER,
            fell: None,
            lock: None,
            trail: None,
            clear: None,
            sparks: Vec::with_capacity(MAX_SPARKS),
            kick: 0.0,
            kicked: NEVER,
            owed: 0.0,
            gain: 0,
            gain_color: 1,
            scored: NEVER,
            levelled: NEVER,
            over: None,
            rng: 0x9E37_79B9,
        }
    }
}

impl Fx {
    /// Starts the effects of one game event.
    pub fn on(&mut self, event: Event) {
        match event {
            Event::Moved { dx, dy } => self.nudge(dx, dy),
            Event::Rotated { dx, dy } => {
                self.nudge(dx, dy);
                // the turn is a quarter clockwise; keep at most a half turn to unwind
                self.spin = (self.spin - FRAC_PI_2).max(-2.0 * FRAC_PI_2);
                self.turned = 0.0;
            }
            Event::HardDropped { rows } => self.fell = Some(rows),
            Event::Locked {
                cells,
                color,
                cleared,
                before,
                points,
            } => self.on_lock(&cells, color, cleared, &before, points),
            Event::LevelUp { level } => {
                self.energy = (level as f32 / 10.0).min(1.0);
                self.levelled = 0.0;
            }
            Event::GameOver => self.over = Some(0.0),
            // heard, not seen: `sound` answers these two
            Event::Blocked | Event::SoftDropped => {}
        }
    }

    /// The piece moved by whole cells; start the drawn piece where it was.
    fn nudge(&mut self, dx: i32, dy: i32) {
        let max = Vec2::splat(SLIDE_MAX);
        self.slide = (self.slide - vec2(dx as f32, dy as f32)).clamp(-max, max);
    }

    fn on_lock(
        &mut self,
        cells: &[(i32, i32)],
        color: u8,
        cleared: Vec<usize>,
        before: &Grid,
        points: u32,
    ) {
        let lively = 1.0 + 0.35 * self.energy;
        let fell = self.fell.take();
        let power = fell.map_or(0.0, drop_power);
        let solid = |x: i32, y: i32| y >= H || before[y as usize][x as usize] != 0;
        // bottom faces of the piece that touch the floor or the stack
        let resting: Vec<(i32, i32)> = cells
            .iter()
            .copied()
            .filter(|&(x, y)| !cells.contains(&(x, y + 1)) && solid(x, y + 1))
            .collect();
        // where a cell is once the cleared rows are gone; `None` if it went with them
        let settle = |(x, y): (i32, i32)| {
            let below = cleared.iter().filter(|&&r| r as i32 > y).count();
            (y < H && !cleared.contains(&(y as usize))).then_some((x as usize, y as usize + below))
        };
        let px = |v: i32| v as f32 * CELL;

        if let Some(rows) = fell.filter(|&rows| rows > 0) {
            let (mut lo, mut hi) = (vec2(f32::MAX, f32::MAX), vec2(f32::MIN, f32::MIN));
            let mut cols: Vec<(f32, f32)> = Vec::new();
            for &(x, y) in cells {
                lo = lo.min(vec2(px(x), px(y)));
                hi = hi.max(vec2(px(x + 1), px(y + 1)));
                match cols.iter_mut().find(|c| c.0 == px(x)) {
                    Some(col) => col.1 = col.1.min(px(y)),
                    None => cols.push((px(x), px(y))),
                }
            }
            self.trail = Some(Trail {
                cols,
                fell: px(rows),
                bounds: Rect::new(lo.x, lo.y, hi.x - lo.x, hi.y - lo.y),
                color,
                power,
                age: 0.0,
            });
            self.knock((1.5 + 2.5 * power) * lively);
            for &(x, y) in resting.iter().take(5) {
                for side in [-1.0, 1.0] {
                    let from = vec2(px(x) + self.range(4.0, CELL - 4.0), px(y + 1) - 2.0);
                    let vel = vec2(
                        side * self.range(50.0, 170.0),
                        -self.range(50.0, 190.0) * (0.5 + 0.5 * power),
                    ) * lively;
                    let (size, life) = (self.range(3.0, 5.0), self.range(0.22, 0.40));
                    self.spark(from, vel, size, life, 0.0, color);
                }
            }
        }

        let kept: Vec<(usize, usize)> = cells.iter().copied().filter_map(settle).collect();
        self.lock = Some(Lock {
            floor: kept.iter().map(|c| c.1 + 1).max().unwrap_or(0) as f32 * CELL,
            cells: kept,
            under: resting
                .iter()
                .filter_map(|&(x, y)| settle((x, y + 1)))
                .collect(),
            contact: if cleared.is_empty() {
                resting.iter().map(|&(x, y)| (px(x), px(y + 1))).collect()
            } else {
                Vec::new() // the row clear speaks for this lock
            },
            carry: self.slide,
            power,
            age: 0.0,
        });
        // the next piece is a new object: it does not inherit the motion of this one
        self.slide = vec2(0.0, SPAWN_SLIDE);
        self.spin = 0.0;
        self.turned = NEVER;

        if !cleared.is_empty() {
            // a clear still in flight gives way to the new one, which starts from where it left
            // the stack
            self.knock((1.0 + cleared.len() as f32) * lively);
            let mut clear = Clear::new(cleared.clone(), cells, before);
            if let Some(old) = &self.clear {
                clear.inherit(old, &cleared);
            }
            self.clear = Some(clear);
        } else if let Some(clear) = &mut self.clear {
            // a clear still in flight is hurried, not cut: the stack is where it really is within
            // a few frames, and the rows still breaking up get to finish
            clear.hurried.get_or_insert(clear.age);
        }
        if points > 0 {
            self.owed = self.owed * (1.0 - ease_out(self.scored / ROLL)) + points as f32;
            self.gain = points;
            self.gain_color = color;
            self.scored = 0.0;
        }
    }

    /// Knocks the well down by `amount` view units, unless a harder knock is still playing.
    fn knock(&mut self, amount: f32) {
        if amount > self.kick_px() {
            self.kick = amount;
            self.kicked = 0.0;
        }
    }

    fn spark(&mut self, from: Vec2, vel: Vec2, size: f32, life: f32, age: f32, color: u8) {
        if self.sparks.len() < MAX_SPARKS && age < life {
            self.sparks.push(Spark {
                from,
                vel,
                size,
                life,
                age,
                color,
            });
        }
    }

    /// Next value of a small xorshift generator, in 0..1.
    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.rand()
    }

    /// Ages every effect by `dt` seconds. Not called while the game is paused.
    pub fn tick(&mut self, dt: f32) {
        // higher levels move faster, so the piece must catch up faster to stay readable
        let quick = 1.0 + 0.5 * self.energy;
        self.slide *= (-SLIDE_RATE * quick * dt).exp();
        self.spin *= (-SPIN_RATE * quick * dt).exp();
        for age in [
            &mut self.turned,
            &mut self.kicked,
            &mut self.scored,
            &mut self.levelled,
        ] {
            *age += dt;
        }
        if let Some(age) = &mut self.over {
            *age += dt;
        }
        if let Some(lock) = &mut self.lock {
            lock.age += dt;
        }
        if let Some(trail) = &mut self.trail {
            trail.age += dt;
        }
        self.lock = self.lock.take().filter(|l| l.age < LOCK_END);
        self.trail = self.trail.take().filter(|t| t.age < TRAIL.max(SHOCK));
        for s in &mut self.sparks {
            s.age += dt;
        }
        self.sparks.retain(|s| s.age < s.life);
        if let Some(mut clear) = self.clear.take() {
            clear.age += dt;
            self.burst(&mut clear);
            self.clear = (clear.age < CLEAR_END).then_some(clear);
        }
    }

    /// Throws the fragments of every cleared cell whose lit time has just ended.
    fn burst(&mut self, clear: &mut Clear) {
        let n = clear.rows.len();
        let per_cell = [2, 3, 3, 5, 5][n.min(5) - 1];
        let force = (0.75 + 0.2 * n as f32) * (1.0 + 0.35 * self.energy);
        let rate = clear.hurried.map_or(1.0, |_| HURRY); // `reached` runs on the hurried clock
        for i in 0..n {
            for x in 0..COLS {
                let late = clear.reached(i, x) - HOLD;
                if late < 0.0 || clear.burst[i] & (1 << x) != 0 {
                    continue;
                }
                clear.burst[i] |= 1 << x;
                let away = (x as f32 + 0.5 - clear.origin[i]).signum();
                for _ in 0..per_cell {
                    let from = vec2(
                        (x as f32 + self.range(0.2, 0.8)) * CELL,
                        (clear.rows[i] as f32 + self.range(0.2, 0.8)) * CELL,
                    );
                    let vel = vec2(
                        away * self.range(20.0, 130.0) + self.range(-45.0, 45.0),
                        -self.range(30.0, 210.0),
                    ) * force;
                    let size = self.range(3.0, 6.5);
                    let life = self.range(0.35, 0.60) + 0.04 * n as f32;
                    // `late` keeps a slow frame from bunching fragments at their start
                    self.spark(from, vel, size, life, late / rate, clear.cells[i][x]);
                }
            }
        }
    }

    // --- what the renderer asks ---

    /// How far down the whole well is drawn, in view units: the knock of an impact, coming back.
    pub fn kick_px(&self) -> f32 {
        self.kick * fade(self.kicked, KICK)
    }

    /// Offset of the drawn falling piece from its real cell in view units, its angle in radians
    /// and how much it is brightened (0..1).
    pub fn piece_pose(&self) -> (Vec2, f32, f32) {
        (
            self.slide * CELL,
            self.spin,
            0.22 * fade(self.turned, TURN_FLASH),
        )
    }

    /// Where the locked cell at column `x`, row `y` is drawn, and how much it is brightened.
    pub fn block(&self, x: usize, y: usize) -> (Rect, f32) {
        // the piece that just locked is where it landed, even if the stack around it is still
        // coming down from an earlier clear
        let fresh = self
            .lock
            .as_ref()
            .is_some_and(|l| l.cells.contains(&(x, y)));
        let dy = self.clear.as_ref().map_or(0.0, |c| c.row_dy(y, fresh));
        let mut r = Rect::new(
            x as f32 * CELL + 1.0,
            y as f32 * CELL + 1.0 + dy,
            CELL - 2.0,
            CELL - 2.0,
        );
        let Some(lock) = &self.lock else {
            return (r, 0.0);
        };
        if lock.cells.contains(&(x, y)) {
            let squash = lock.squash();
            let floor = lock.floor + dy;
            r.y = floor - (floor - r.y) * (1.0 - squash);
            r.h *= 1.0 - squash;
            let bulge = (squash * CELL * 0.2).min(1.0); // never past the gap to the next cell
            r.x -= bulge;
            r.w += 2.0 * bulge;
            let carry = lock.carry * CELL * (-SLIDE_RATE * lock.age).exp();
            r.x += carry.x;
            r.y += carry.y;
            (r, (0.22 + 0.45 * lock.power) * fade(lock.age, LOCK_FLASH))
        } else if lock.under.contains(&(x, y)) {
            (
                r,
                (0.06 + 0.16 * lock.power) * fade(lock.age, LOCK_FLASH * 1.3),
            )
        } else {
            (r, 0.0)
        }
    }

    /// The score to show: the real one, minus what the count-up has not reached yet.
    pub fn score(&self, actual: u32) -> u32 {
        let behind = self.owed * (1.0 - ease_out(self.scored / ROLL));
        actual.saturating_sub(behind.round() as u32)
    }

    /// Colour laid over the score while it counts up, already faded.
    pub fn score_tint(&self) -> Color {
        alpha(piece_color(self.gain_color), fade(self.scored, ROLL + 0.25))
    }

    /// The "+points" label: its value, its colour already faded, and how far it has risen.
    pub fn gain(&self) -> Option<(u32, Color, f32)> {
        (self.scored < GAIN).then(|| {
            let shown =
                ease_out(self.scored / 0.08) * (1.0 - ease_in_out((self.scored - 0.65) / 0.35));
            (
                self.gain,
                alpha(piece_color(self.gain_color), shown),
                4.0 * ease_out(self.scored / 0.3),
            )
        })
    }

    /// Seconds since the level last rose; very large if it never did.
    pub fn levelled(&self) -> f32 {
        self.levelled
    }

    /// How dark row `row` of the well is at the end of the game (0..1). The final state is held
    /// first, then the dimming runs from the top row down.
    pub fn over_dim(&self, row: usize) -> f32 {
        // no event seen (a game built already over): fully dimmed
        self.over.map_or(1.0, |age| {
            ease_in_out((age - OVER_FREEZE - row as f32 * OVER_ROW_LAG) / OVER_DIM)
        })
    }

    /// How far the game-over box has come in (0..1).
    pub fn over_box(&self) -> f32 {
        self.over
            .map_or(1.0, |age| ease_out((age - OVER_BOX_AT) / OVER_BOX))
    }

    // --- drawing; all of it inside the well ---

    /// Light on the floor of the well, under every block.
    pub fn draw_under(&self) {
        if let Some(c) = &self.clear {
            let n = c.rows.len() as f32;
            // the whole well lifts for a moment
            let lift = 0.025 * n * fade(c.age, 0.3);
            draw_rectangle(0.0, 0.0, WELL_W, WELL_H, alpha(WHITE, lift));
            // a line leaves the cleared rows upward and another downward
            let reach = (50.0 + 55.0 * n) * ease_out(c.age / RIPPLE);
            let a = (0.05 + 0.035 * n) * fade(c.age, RIPPLE);
            let top = c.rows[0] as f32 * CELL - reach;
            let bottom = (*c.rows.last().unwrap() + 1) as f32 * CELL + reach;
            for (y, toward) in [(top, 1.0), (bottom, -1.0)] {
                draw_rectangle(0.0, y - 1.0, WELL_W, 2.0, alpha(WHITE, a));
                // a soft wake behind the line, on the side it came from
                for k in 0..4 {
                    let step = 4.0 * k as f32 * toward;
                    let wake = a * 0.3 * (1.0 - k as f32 / 4.0);
                    draw_rectangle(0.0, y + step - 2.0, WELL_W, 4.0, alpha(WHITE, wake));
                }
            }
        }
        if self.levelled < LEVEL_SWEEP {
            // a band in the level colour rises through the well
            let p = self.levelled / LEVEL_SWEEP;
            let y = WELL_H - (WELL_H + 60.0) * ease_in_out(p);
            for k in 0..5 {
                let a = 0.10 * (1.0 - k as f32 / 5.0) * (1.0 - p);
                draw_rectangle(0.0, y + 12.0 * k as f32, WELL_W, 12.0, alpha(COLORS[1], a));
            }
        }
        if let Some(t) = self.trail.as_ref().filter(|t| t.age < TRAIL) {
            // a streak above each column of the piece, catching up with it
            let p = t.age / TRAIL;
            let peak = (0.10 + 0.24 * t.power) * (1.0 + 0.3 * self.energy) * (1.0 - p);
            const STEPS: usize = 8;
            for &(x, y) in &t.cols {
                let head = y - t.fell * (1.0 - ease_out(p));
                let step = (y - head) / STEPS as f32;
                for k in 0..STEPS {
                    let a = peak * ((k + 1) as f32 / STEPS as f32).powi(2);
                    draw_rectangle(
                        x + 4.0,
                        head + step * k as f32,
                        CELL - 8.0,
                        step,
                        alpha(piece_color(t.color), a),
                    );
                }
            }
        }
    }

    /// The cleared rows breaking up, drawn where they were, over the stack.
    pub fn draw_clear(&self) {
        let Some(c) = &self.clear else { return };
        for (i, &row) in c.rows.iter().enumerate() {
            let y = row as f32 * CELL;
            for x in 0..COLS {
                let base = piece_color(c.cells[i][x]);
                let t = c.reached(i, x);
                // waiting for the sweep, then lit, then shrinking to nothing
                let (color, scale) = if t < 0.0 {
                    (base, 1.0)
                } else if t < HOLD {
                    (mix(base, WHITE, 0.9 * ease_out(t / 0.03)), 1.0)
                } else if t < HOLD + BURST {
                    (mix(base, WHITE, 0.9), 1.0 - ((t - HOLD) / BURST).powi(2))
                } else {
                    continue;
                };
                let side = (CELL - 2.0) * scale;
                let inset = (CELL - side) / 2.0;
                draw_rectangle(x as f32 * CELL + inset, y + inset, side, side, color);
            }
            // the two fronts of the sweep, leaving the column where the piece closed the row
            let since = c.clock() - (c.rows.len() - 1 - i) as f32 * ROW_LAG;
            if since < 0.0 {
                continue;
            }
            let run = since * SWEEP_SPEED * CELL;
            let start = c.origin[i] * CELL;
            for side in [-1.0, 1.0] {
                let front = start + side * run;
                if (0.0..=WELL_W).contains(&front) {
                    draw_rectangle(front - 6.0, y, 12.0, CELL, alpha(WHITE, 0.22));
                    draw_rectangle(front - 1.5, y - 2.0, 3.0, CELL + 4.0, alpha(WHITE, 0.95));
                }
            }
            // a seam of light where the row was, closing as the stack comes down
            let (left, right) = ((start - run).max(0.0), (start + run).min(WELL_W));
            let thick = 2.0 * fade(since, 0.32);
            let seam = alpha(WHITE, 0.5 * fade(since, 0.32));
            draw_rectangle(left, y + (CELL - thick) / 2.0, right - left, thick, seam);
        }
    }

    /// Light and fragments over everything else in the well.
    pub fn draw_over(&self) {
        if let Some(lock) = &self.lock {
            // a line of light along each edge that made contact, spreading as it fades
            let p = lock.age / 0.18;
            if p < 1.0 {
                let spread = 5.0 * ease_out(p);
                let a = (0.45 + 0.4 * lock.power) * (1.0 - p).powi(2);
                for &(x, y) in &lock.contact {
                    let (x0, x1) = ((x - spread).max(0.0), (x + CELL + spread).min(WELL_W));
                    draw_rectangle(x0, y.min(WELL_H) - 2.0, x1 - x0, 2.0, alpha(WHITE, a));
                }
            }
        }
        if let Some(t) = self.trail.as_ref().filter(|t| t.age < SHOCK) {
            // a ring the shape of the piece's bounds, leaving the landing spot
            let p = t.age / SHOCK;
            let grow = (6.0 + 16.0 * t.power) * ease_out(p);
            let a = (0.2 + 0.35 * t.power) * (1.0 - p).powi(2);
            let b = t.bounds;
            draw_rectangle_lines(
                b.x - grow,
                b.y - grow,
                b.w + 2.0 * grow,
                b.h + 2.0 * grow,
                2.0,
                alpha(piece_color(t.color), a),
            );
        }
        for s in &self.sparks {
            let p = s.age / s.life;
            let at = s.from + s.vel * s.age + vec2(0.0, 0.5 * GRAVITY * s.age * s.age);
            let size = s.size * (1.0 - 0.6 * p);
            // fragments stay inside the well: none may cross into the side panel
            if at.x < size / 2.0 || at.x > WELL_W - size / 2.0 || at.y > WELL_H {
                continue;
            }
            // white-hot as it leaves, then the colour of the cell it came from
            let color = mix(WHITE, piece_color(s.color), ease_out(s.age / 0.14));
            draw_rectangle(
                at.x - size / 2.0,
                at.y - size / 2.0,
                size,
                size,
                alpha(color, (1.0 - p).powf(1.5)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTTOM: usize = ROWS - 1;

    /// A lock that fills and removes `rows` (ascending) with a one-column bar at column 0.
    /// Row `rows[0] - 1` holds one stray cell at column 5, so there is something to bring down.
    fn clear_event(rows: &[usize]) -> Event {
        let mut before: Grid = [[0; COLS]; ROWS];
        for &r in rows {
            before[r] = [1; COLS];
        }
        before[rows[0] - 1][5] = 2;
        Event::Locked {
            cells: rows.iter().map(|&r| (0, r as i32)).collect(),
            color: 2,
            cleared: rows.to_vec(),
            before: Box::new(before),
            points: (rows.len() * rows.len()) as u32 * 100,
        }
    }

    fn settled_lock(cells: Vec<(i32, i32)>, before: Grid) -> Event {
        Event::Locked {
            cells,
            color: 1,
            cleared: Vec::new(),
            before: Box::new(before),
            points: 0,
        }
    }

    #[test]
    fn rows_above_a_clear_start_where_they_were_and_end_in_place() {
        let mut fx = Fx::default();
        fx.on(clear_event(&[BOTTOM - 1, BOTTOM]));
        // the stray cell was on row BOTTOM - 2 and is on row BOTTOM in the grid now
        assert_eq!(fx.block(5, BOTTOM).0.y, (BOTTOM - 2) as f32 * CELL + 1.0);
        // an empty row at the top came down too, but nothing under the clear exists to move
        assert_eq!(fx.block(0, 0).0.y, 1.0 - 0.0);
        fx.tick(COLLAPSE_AT + COLLAPSE_LAG * 8.0 + COLLAPSE + 0.001);
        assert_eq!(fx.block(5, BOTTOM).0.y, BOTTOM as f32 * CELL + 1.0);
    }

    #[test]
    fn rows_between_two_cleared_rows_come_down_by_one() {
        let mut fx = Fx::default();
        let mut before: Grid = [[0; COLS]; ROWS];
        before[BOTTOM] = [1; COLS];
        before[BOTTOM - 1][3] = 1;
        before[BOTTOM - 2] = [1; COLS];
        before[BOTTOM - 3][4] = 1;
        fx.on(Event::Locked {
            cells: vec![
                (0, BOTTOM as i32),
                (0, BOTTOM as i32 - 1),
                (0, BOTTOM as i32 - 2),
            ],
            color: 1,
            cleared: vec![BOTTOM - 2, BOTTOM],
            before: Box::new(before),
            points: 400,
        });
        let c = fx.clear.as_ref().unwrap();
        assert_eq!((c.fall[BOTTOM], c.fall[BOTTOM - 1]), (1, 2));
        // the cell of the piece that survived is tracked where it is now, one row lower
        assert_eq!(fx.lock.as_ref().unwrap().cells, vec![(0, BOTTOM)]);
    }

    #[test]
    fn a_lock_during_a_clear_hurries_it_without_a_jump() {
        let empty: Grid = [[0; COLS]; ROWS];
        let per_cell = 3; // two rows
        for interrupt_at in [0.06, 0.13, 0.22] {
            let mut fx = Fx::default();
            fx.on(clear_event(&[BOTTOM - 1, BOTTOM]));
            fx.tick(interrupt_at);
            let before = fx.block(5, BOTTOM).0.y;
            let thrown = fx.sparks.len();
            fx.on(settled_lock(vec![(9, BOTTOM as i32)], empty));
            // the stack is drawn exactly where it was the frame before: nothing jumps
            assert_eq!(fx.block(5, BOTTOM).0.y, before);
            assert!(before < BOTTOM as f32 * CELL + 1.0);
            // the piece that locked is drawn where it landed, give or take the last of its own
            // slide, and not up with the rows that are still coming down
            assert!((fx.block(9, BOTTOM).0.y - (BOTTOM as f32 * CELL + 1.0)).abs() < 2.0);
            // four frames later the stack is where it really is
            let mut last = before;
            for _ in 0..4 {
                fx.tick(1.0 / 60.0);
                let y = fx.block(5, BOTTOM).0.y;
                assert!(y >= last);
                last = y;
            }
            assert_eq!(last, BOTTOM as f32 * CELL + 1.0);
            // and every cell still got to break up
            assert!(fx.sparks.len() >= thrown);
            fx.tick(0.02);
            let burst: u32 = fx
                .clear
                .as_ref()
                .unwrap()
                .burst
                .iter()
                .map(|b| b.count_ones())
                .sum();
            assert_eq!(burst as usize, 2 * COLS);
            assert!(fx.sparks.len() <= 2 * COLS * per_cell);
        }
    }

    #[test]
    fn a_clear_during_a_clear_takes_over_without_a_jump() {
        for interrupt_at in [0.1, 0.22] {
            let mut fx = Fx::default();
            fx.on(clear_event(&[BOTTOM - 1, BOTTOM]));
            fx.tick(interrupt_at);
            // where a cell on the row above the bottom one is drawn while the stack comes down
            let before = fx.block(5, BOTTOM - 1).0.y;
            assert!(before < (BOTTOM - 1) as f32 * CELL + 1.0);
            // the bottom row goes next, with a cell on the row above it
            let mut grid: Grid = [[0; COLS]; ROWS];
            grid[BOTTOM] = [1; COLS];
            grid[BOTTOM - 1][5] = 2;
            fx.on(Event::Locked {
                cells: vec![(0, BOTTOM as i32)],
                color: 1,
                cleared: vec![BOTTOM],
                before: Box::new(grid),
                points: 100,
            });
            let c = fx.clear.as_ref().unwrap();
            assert_eq!((c.rows.len(), c.age, c.hurried), (1, 0.0, None));
            // that cell is on the bottom row of the grid now, and drawn exactly where it was
            assert!((fx.block(5, BOTTOM).0.y - before).abs() < 1e-3);
            fx.tick(INHERIT);
            assert_eq!(fx.block(5, BOTTOM).0.y, (BOTTOM - 1) as f32 * CELL + 1.0);
            fx.tick(1.0);
            assert_eq!(fx.block(5, BOTTOM).0.y, BOTTOM as f32 * CELL + 1.0);
        }
    }

    #[test]
    fn irregular_frames_end_in_the_same_place() {
        // the same 0.3 s in even steps and in steps from 1 to 60 ms, including a lock mid-clear
        let run = |steps: &[f32]| {
            let empty: Grid = [[0; COLS]; ROWS];
            let mut fx = Fx::default();
            fx.on(Event::HardDropped { rows: 9 });
            fx.on(clear_event(&[BOTTOM - 2, BOTTOM - 1, BOTTOM]));
            let mut t = 0.0;
            let mut interrupted = false;
            for &dt in steps {
                fx.tick(dt);
                t += dt;
                if t >= 0.12 - 1e-4 && !interrupted {
                    fx.on(settled_lock(vec![(9, BOTTOM as i32)], empty));
                    interrupted = true;
                }
            }
            assert!((t - 0.3).abs() < 1e-4);
            (
                fx.block(5, BOTTOM).0.y,
                fx.score(900),
                fx.kick_px(),
                fx.sparks.len(),
            )
        };
        let even = run(&[0.02; 15]);
        let rough = run(&[0.001, 0.059, 0.016, 0.044, 0.003, 0.057, 0.033, 0.027, 0.06]);
        assert_eq!((even.0, even.1, even.3), (rough.0, rough.1, rough.3));
        assert!((even.2 - rough.2).abs() < 1e-3);
    }

    #[test]
    fn nothing_is_left_running_after_a_long_frame() {
        let mut fx = Fx::default();
        fx.on(Event::Moved { dx: 1, dy: 0 });
        fx.on(Event::Rotated { dx: 0, dy: 0 });
        fx.on(Event::HardDropped { rows: 15 });
        fx.on(clear_event(&[BOTTOM - 1, BOTTOM]));
        fx.on(Event::LevelUp { level: 1 });
        fx.tick(5.0); // one frame, as after a stalled window
        assert!(fx.clear.is_none() && fx.lock.is_none() && fx.trail.is_none());
        assert!(fx.sparks.is_empty() && fx.gain().is_none());
        let (slide, spin, flash) = fx.piece_pose();
        assert!(slide.length() < 1e-3 && spin.abs() < 1e-3 && flash == 0.0);
        assert_eq!(
            (fx.kick_px(), fx.score(400), fx.block(5, BOTTOM).1),
            (0.0, 400, 0.0)
        );
        assert!(fx.levelled() > LEVEL_SWEEP);
    }

    #[test]
    fn every_cleared_cell_breaks_up_exactly_once() {
        for n in 1..=5 {
            let rows: Vec<usize> = (ROWS - n..ROWS).collect();
            let per_cell = [2, 3, 3, 5, 5][n - 1];
            let mut fx = Fx::default();
            fx.on(clear_event(&rows));
            let mut most = 0;
            for _ in 0..40 {
                fx.tick(0.005);
                most = most.max(fx.sparks.len());
                assert!(fx.sparks.len() <= MAX_SPARKS);
            }
            // by 0.2 s every cell has burst and no fragment has died yet
            assert_eq!(most, n * COLS * per_cell);
            fx.tick(2.0);
            assert!(fx.sparks.is_empty() && fx.clear.is_none());
        }
    }

    #[test]
    fn a_bigger_clear_is_a_bigger_event() {
        let mut last = (0, 0.0);
        for n in 1..=4 {
            let rows: Vec<usize> = (ROWS - n..ROWS).collect();
            let mut fx = Fx::default();
            fx.on(clear_event(&rows));
            let kick = fx.kick_px();
            fx.tick(0.2);
            assert!(fx.sparks.len() > last.0 && kick > last.1);
            last = (fx.sparks.len(), kick);
        }
    }

    #[test]
    fn effects_do_not_depend_on_the_frame_rate() {
        let run = |steps: usize| {
            let mut fx = Fx::default();
            fx.on(Event::Moved { dx: 1, dy: 0 });
            fx.on(Event::Rotated { dx: 0, dy: 0 });
            fx.on(clear_event(&[BOTTOM]));
            for _ in 0..steps {
                fx.tick(0.1 / steps as f32);
            }
            let at: Vec<Vec2> = fx
                .sparks
                .iter()
                .map(|s| s.from + s.vel * s.age + vec2(0.0, 0.5 * GRAVITY * s.age * s.age))
                .collect();
            (fx.piece_pose().0, fx.score(100), fx.kick_px(), at)
        };
        let (slow, fast) = (run(3), run(24)); // 30 and 240 frames a second
        assert!((slow.0 - fast.0).length() < 1e-3);
        assert_eq!(slow.1, fast.1);
        assert!((slow.2 - fast.2).abs() < 1e-3);
        assert_eq!(slow.3.len(), fast.3.len());
        for (a, b) in slow.3.iter().zip(&fast.3) {
            assert!((*a - *b).length() < 0.5, "{a} vs {b}");
        }
    }

    #[test]
    fn the_drawn_piece_starts_where_it_was_and_catches_up() {
        let mut fx = Fx::default();
        fx.on(Event::Moved { dx: 1, dy: 0 });
        assert_eq!(fx.piece_pose().0, vec2(-CELL, 0.0));
        fx.on(Event::Rotated { dx: -1, dy: 0 });
        let (slide, spin, flash) = fx.piece_pose();
        assert_eq!((slide, spin), (Vec2::ZERO, -FRAC_PI_2));
        assert!(flash > 0.0);
        fx.tick(0.3);
        let (slide, spin, flash) = fx.piece_pose();
        assert!(slide.length() < 0.01 && spin.abs() < 0.001 && flash == 0.0);
    }

    #[test]
    fn only_a_hard_drop_leaves_a_trail_and_knocks_the_well() {
        let empty: Grid = [[0; COLS]; ROWS];
        let cells = vec![(3, BOTTOM as i32), (4, BOTTOM as i32)];
        let mut fx = Fx::default();
        fx.on(settled_lock(cells.clone(), empty));
        assert!(fx.trail.is_none() && fx.sparks.is_empty() && fx.kick_px() == 0.0);
        let soft = fx.block(3, BOTTOM).1;

        fx.on(Event::HardDropped { rows: 12 });
        fx.on(settled_lock(cells.clone(), empty));
        assert!(fx.trail.is_some() && !fx.sparks.is_empty() && fx.kick_px() > 0.0);
        assert!(fx.block(3, BOTTOM).1 > soft);
        // both cells touched the floor
        assert_eq!(fx.lock.as_ref().unwrap().contact.len(), 2);

        // the hard drop was used up by its lock
        fx.on(settled_lock(cells, empty));
        assert!(fx.lock.as_ref().unwrap().power == 0.0);
    }

    #[test]
    fn the_shown_score_counts_up_to_the_real_one_and_never_past_it() {
        let mut fx = Fx::default();
        assert_eq!(fx.score(700), 700);
        fx.on(clear_event(&[BOTTOM - 1, BOTTOM])); // 400 points, so the real score is now 1100
        assert_eq!(fx.score(1100), 700);
        assert_eq!(fx.gain().unwrap().0, 400);
        let mut last = 700;
        for _ in 0..60 {
            fx.tick(0.01);
            let shown = fx.score(1100);
            assert!((last..=1100).contains(&shown));
            last = shown;
        }
        assert_eq!(last, 1100);
        fx.tick(1.0);
        assert!(fx.gain().is_none());
    }

    #[test]
    fn the_end_is_held_then_dimmed_from_the_top_down() {
        let mut fx = Fx::default();
        fx.on(Event::GameOver);
        fx.tick(OVER_FREEZE - 0.01);
        assert_eq!(
            (fx.over_dim(0), fx.over_dim(BOTTOM), fx.over_box()),
            (0.0, 0.0, 0.0)
        );
        fx.tick(0.2);
        assert!(fx.over_dim(0) > fx.over_dim(BOTTOM) && fx.over_box() == 0.0);
        fx.tick(2.0);
        assert_eq!(
            (fx.over_dim(0), fx.over_dim(BOTTOM), fx.over_box()),
            (1.0, 1.0, 1.0)
        );
        // a game that never sent the event is shown fully dimmed, not stuck undimmed
        assert_eq!(Fx::default().over_box(), 1.0);
    }

    #[test]
    fn levels_raise_the_energy_up_to_a_ceiling() {
        let mut fx = Fx::default();
        assert!(fx.levelled() > 100.0);
        fx.on(Event::LevelUp { level: 5 });
        assert_eq!((fx.energy, fx.levelled()), (0.5, 0.0));
        fx.on(Event::LevelUp { level: 30 });
        assert_eq!(fx.energy, 1.0);
    }
}
