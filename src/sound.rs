//! Sound: which cue answers which event, how loud, and what the music does meanwhile.
//!
//! Like `fx`, this module only listens. The game pushes an [`Event`], `Desk::on` decides what
//! is heard, and nothing here is read back by game logic, so sound cannot change a rule, a
//! score or a timing.
//!
//! Deciding and playing are kept apart. `Desk` is the mixing desk: plain computation on events
//! and elapsed time, tested without a sound card. It writes `Order`s. `Speakers` carries them
//! out through macroquad and is the only part that touches the audio device.
//!
//! Two facts about macroquad's mixer shape every rule below. It starts sounds only at the
//! beginning of a block of 4096 samples, 93 ms apart, so two sounds asked for within one block
//! start together. And it adds everything up with no limiter, so the levels below must leave
//! the room themselves.

use crate::{fx, Event};
use macroquad::audio::{
    load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound,
};

/// A file from `assets/`, compiled into the binary once. The `static` matters: a bare
/// `include_bytes!` in a function is copied into the binary for every place the compiler
/// inlines that function, and the theme alone is 1.8 MB.
macro_rules! asset {
    ($path:literal) => {{
        static BYTES: &[u8] = include_bytes!(concat!("../assets/", $path));
        BYTES
    }};
}

/// Everything the game can play. The files are rendered from `audio/` by `mise run audio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    // music
    Title,
    Theme,
    LevelUp,
    GameOver,
    // contact: what the piece physically does
    MoveLeft,
    MoveRight,
    Rotate,
    Blocked,
    SoftDrop,
    Lock,
    HardDrop,
    // glass: what the rules do
    Clear1,
    Clear2,
    Clear3,
    Clear4,
    Clear5,
    Pause,
    Resume,
    NewGame,
}

impl Cue {
    const ALL: [Cue; 19] = [
        Cue::Title,
        Cue::Theme,
        Cue::LevelUp,
        Cue::GameOver,
        Cue::MoveLeft,
        Cue::MoveRight,
        Cue::Rotate,
        Cue::Blocked,
        Cue::SoftDrop,
        Cue::Lock,
        Cue::HardDrop,
        Cue::Clear1,
        Cue::Clear2,
        Cue::Clear3,
        Cue::Clear4,
        Cue::Clear5,
        Cue::Pause,
        Cue::Resume,
        Cue::NewGame,
    ];
    const CLEARS: [Cue; 5] = [
        Cue::Clear1,
        Cue::Clear2,
        Cue::Clear3,
        Cue::Clear4,
        Cue::Clear5,
    ];

    /// The file, compiled into the binary. The soft drop is a loop and must keep its exact
    /// length, so the effects are WAV: the Ogg decoder pads the end of a short file.
    fn bytes(self) -> &'static [u8] {
        match self {
            Cue::Title => asset!("title.ogg"),
            Cue::Theme => asset!("theme.ogg"),
            Cue::LevelUp => asset!("level_up.ogg"),
            Cue::GameOver => asset!("game_over.ogg"),
            Cue::MoveLeft => asset!("sfx/move_left.wav"),
            Cue::MoveRight => asset!("sfx/move_right.wav"),
            Cue::Rotate => asset!("sfx/rotate.wav"),
            Cue::Blocked => asset!("sfx/blocked.wav"),
            Cue::SoftDrop => asset!("sfx/soft_drop.wav"),
            Cue::Lock => asset!("sfx/lock.wav"),
            Cue::HardDrop => asset!("sfx/hard_drop.wav"),
            Cue::Clear1 => asset!("sfx/clear_1.wav"),
            Cue::Clear2 => asset!("sfx/clear_2.wav"),
            Cue::Clear3 => asset!("sfx/clear_3.wav"),
            Cue::Clear4 => asset!("sfx/clear_4.wav"),
            Cue::Clear5 => asset!("sfx/clear_5.wav"),
            Cue::Pause => asset!("sfx/pause.wav"),
            Cue::Resume => asset!("sfx/resume.wav"),
            Cue::NewGame => asset!("sfx/new_game.wav"),
        }
    }
}

/// The mix: how loud each cue plays, as a factor on its file. Every effect file and the level-up
/// peak at -3 dBFS, so the numbers in this table alone set the balance. To move a sound up or
/// down, change its number here; nothing needs rendering again. The decibels are the same
/// factor, for reading. `MASTER` then lowers them all together.
fn level(cue: Cue) -> f32 {
    let balance = match cue {
        Cue::Title => 0.60,   //  -4.4 dB: the title screen's theme, as loud as the game's
        Cue::Theme => 0.60,   //  -4.4 dB: the bed; leaves the room the effects need
        Cue::LevelUp => 0.40, //  -8 dB
        Cue::GameOver => 0.70, //  -3 dB
        Cue::SoftDrop => 0.16, // -16 dB: the quietest sound in the game
        Cue::MoveLeft => 0.32, // -10 dB
        Cue::MoveRight => 0.32, // -10 dB
        Cue::Rotate => 0.40,  //  -8 dB
        Cue::Blocked => 0.40, //  -8 dB: its tap carries it on small speakers
        Cue::Lock => 0.53,    //  -5.5 dB
        Cue::HardDrop => 0.79, //  -2 dB, for the longest fall
        Cue::Clear1 => 0.35,  //  -9 dB
        Cue::Clear2 => 0.38,  //  -8.5 dB
        Cue::Clear3 => 0.45,  //  -7 dB
        Cue::Clear4 => 0.70,  //  -3 dB
        Cue::Clear5 => 0.68,  //  -3.3 dB
        Cue::Pause => 0.20,   // -14 dB
        Cue::Resume => 0.20,  // -14 dB
        Cue::NewGame => 0.22, // -13 dB
    };
    balance * MASTER
}

/// Room for sounds to pile up. The mixer adds every playing sound and has no limiter: what the
/// levels do not leave, the sound card clips. A lock, its hard drop, a row clear, a level-up and
/// a key or two pressed in the same instant all start on one sample, and at the balance alone
/// they pass full scale. With this factor on everything they stay under it, wherever in the
/// theme they fall. `tools/sound/bench.py check` tries every case; run it after changing a level.
/// The whole game plays this much quieter; the balance between its sounds is untouched.
const MASTER: f32 = 0.71; // -3 dB

// Two moves closer than one mixer block would start on the same sample and be heard as one
// tick twice as loud, so a held key ticks on every other column instead.
const MOVE_GAP: f32 = 0.09;
// A move this soon after the one before is a held key repeating (every 50 ms), not a new press.
const REPEAT_WINDOW: f32 = 0.075;
const REPEAT_LEVEL: f32 = 0.7; // repeats are quieter than the press that started them
const SOFT_TIMEOUT: f32 = 0.15; // seconds without a soft-drop row before the run stops
const DUCK_HOLD: f32 = 0.25; // seconds the theme stays down under a row clear
const DUCK_RELEASE: f32 = 5.0; // dB per second on the way back
const FADE_OUT: f32 = 0.2; // seconds the theme takes to leave on a pause
const FADE_IN: f32 = 0.3; // and to return
const TITLE_FADE: f32 = 1.2; // seconds the title theme takes to leave once a game starts

/// Decibels the theme drops by while rows clear, by rows cleared at once.
const CLEAR_DUCK: [f32; 5] = [0.0, -1.5, -3.0, -5.0, -6.0];
/// The level-up cue hangs for two seconds: the theme makes room for all of it.
const LEVEL_UP_DUCK: (f32, f32) = (-4.0, 1.8);

/// What the desk asks of the speakers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Order {
    /// Play the cue once from its start at this volume. It may overlap itself.
    Play(Cue, f32),
    /// Play the cue round and round at this volume, until it is stopped.
    Loop(Cue, f32),
    /// Change the volume of the cue wherever it is playing.
    Volume(Cue, f32),
    /// Silence every playing copy of the cue.
    Stop(Cue),
}

fn gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub struct Desk {
    orders: Vec<Order>,
    since_move: f32,   // seconds since a move was last heard
    since_step: f32,   // seconds since the piece last moved sideways, heard or not
    running: bool,     // the soft-drop loop is playing
    since_soft: f32,   // seconds since the last soft-drop row, counted from the frame after it
    fell: Option<i32>, // rows of the hard drop whose lock is about to arrive
    duck: f32,         // dB off the theme now, <= 0
    duck_hold: f32,    // seconds before the duck starts to come back
    paused: bool,
    fade: f32,  // the theme around a pause: 1 playing, 0 paused
    over: bool, // the end of the game has stopped the theme
    music: f32, // the theme volume last ordered
    title: f32, // the title theme, as a factor on its level: 1 on the title screen, then fading
    title_fading: bool,
}

impl Desk {
    /// A desk with the theme already asked for.
    pub fn new() -> Self {
        let mut desk = Desk {
            orders: Vec::with_capacity(Cue::ALL.len() + 4),
            since_move: MOVE_GAP,
            since_step: REPEAT_WINDOW,
            running: false,
            since_soft: 0.0,
            fell: None,
            duck: 0.0,
            duck_hold: 0.0,
            paused: false,
            fade: 1.0,
            over: false,
            music: level(Cue::Theme),
            title: 0.0,
            title_fading: false,
        };
        desk.orders.push(Order::Loop(Cue::Theme, desk.music));
        desk
    }

    /// A desk for the title screen: the title theme plays, and nothing of a game.
    pub fn title() -> Self {
        let mut desk = Desk::new();
        desk.orders.clear();
        desk.orders.push(Order::Loop(Cue::Title, level(Cue::Title)));
        desk.over = true; // no game theme to steer
        desk.title = 1.0;
        desk
    }

    /// Back to the title screen: whatever the game left ringing stops, the title theme starts.
    pub fn back_to_title(&mut self) {
        let mut orders = std::mem::take(&mut self.orders);
        orders.clear();
        orders.extend(Cue::ALL.map(Order::Stop));
        *self = Desk::title();
        orders.append(&mut self.orders);
        self.orders = orders;
    }

    /// The orders written since this was last called, oldest first.
    pub fn orders(&mut self) -> std::vec::Drain<'_, Order> {
        self.orders.drain(..)
    }

    fn play(&mut self, cue: Cue, factor: f32) {
        self.orders.push(Order::Play(cue, level(cue) * factor));
    }

    /// Lowers the theme by `depth` dB for `hold` seconds. Of two ducks, the deeper and the
    /// longer win; the theme then comes back at `DUCK_RELEASE`.
    fn duck(&mut self, depth: f32, hold: f32) {
        if depth < 0.0 {
            self.duck = self.duck.min(depth);
            self.duck_hold = self.duck_hold.max(hold);
        }
    }

    fn stop_run(&mut self) {
        if self.running {
            self.running = false;
            self.orders.push(Order::Stop(Cue::SoftDrop));
        }
    }

    /// Answers one game event.
    pub fn on(&mut self, event: &Event) {
        match event {
            Event::Moved { dx, .. } if *dx != 0 => {
                let repeat = self.since_step < REPEAT_WINDOW;
                self.since_step = 0.0;
                if self.since_move >= MOVE_GAP {
                    self.since_move = 0.0;
                    let cue = if *dx < 0 {
                        Cue::MoveLeft
                    } else {
                        Cue::MoveRight
                    };
                    self.play(cue, if repeat { REPEAT_LEVEL } else { 1.0 });
                }
            }
            // a row down: gravity is silent, and a soft drop announces itself
            Event::Moved { .. } => {}
            Event::SoftDropped => {
                // negative: the frame this row fell in is not time without a row, however long
                // it was. One slow frame must not stop a run that is still going.
                self.since_soft = f32::NEG_INFINITY;
                if !self.running {
                    self.running = true;
                    self.orders
                        .push(Order::Loop(Cue::SoftDrop, level(Cue::SoftDrop)));
                }
            }
            Event::Rotated { .. } => self.play(Cue::Rotate, 1.0),
            Event::Blocked => self.play(Cue::Blocked, 1.0),
            Event::HardDropped { rows } => self.fell = Some(*rows),
            Event::Locked { cells, cleared, .. } => {
                self.stop_run(); // the piece has landed
                                 // a bigger piece lands a little heavier
                let weight = match cells.len() {
                    0..=3 => 0.84,
                    4 => 0.92,
                    _ => 1.0,
                };
                self.play(Cue::Lock, weight);
                // a hard drop adds its thump on top, scaled like the trail that is drawn
                if let Some(rows) = self.fell.take() {
                    self.play(Cue::HardDrop, fx::drop_power(rows));
                }
                if !cleared.is_empty() {
                    let n = cleared.len().min(5) - 1;
                    self.play(Cue::CLEARS[n], 1.0);
                    self.duck(CLEAR_DUCK[n], DUCK_HOLD);
                }
            }
            Event::LevelUp { .. } => {
                self.play(Cue::LevelUp, 1.0);
                self.duck(LEVEL_UP_DUCK.0, LEVEL_UP_DUCK.1);
            }
            Event::GameOver => {
                self.stop_run();
                self.over = true;
                // the ending replaces the music: a level-up cue from the same lock goes too
                self.orders.push(Order::Stop(Cue::Theme));
                self.orders.push(Order::Stop(Cue::LevelUp));
                self.play(Cue::GameOver, 1.0);
            }
        }
    }

    /// Once a frame, paused or not: the pause itself, the end of a soft drop, and the theme's
    /// volume as it ducks and fades.
    pub fn frame(&mut self, paused: bool, dt: f32) {
        self.since_move += dt;
        self.since_step += dt;
        if paused != self.paused {
            self.paused = paused;
            if paused {
                // the game is frozen: nothing it started may ring on over the overlay. The theme
                // has nothing left to make room for, so its lowering is held no longer; but it
                // lets go at its own pace, or the theme would jump up before it fades out
                self.running = false;
                self.duck_hold = 0.0;
                for cue in Cue::ALL {
                    if cue != Cue::Theme {
                        self.orders.push(Order::Stop(cue));
                    }
                }
                self.play(Cue::Pause, 1.0);
            } else {
                self.play(Cue::Resume, 1.0);
            }
        }
        if self.running {
            self.since_soft = (self.since_soft + dt).max(0.0);
            if self.since_soft > SOFT_TIMEOUT {
                self.stop_run();
            }
        }
        if self.duck_hold > 0.0 {
            self.duck_hold -= dt;
        } else {
            self.duck = (self.duck + DUCK_RELEASE * dt).min(0.0);
        }
        // macroquad cannot pause one sound, so the theme fades out and plays on unheard
        self.fade = if paused {
            (self.fade - dt / FADE_OUT).max(0.0)
        } else {
            (self.fade + dt / FADE_IN).min(1.0)
        };
        if self.title_fading {
            self.title = (self.title - dt / TITLE_FADE).max(0.0);
            if self.title == 0.0 {
                self.title_fading = false;
                self.orders.push(Order::Stop(Cue::Title));
            } else {
                // equal power: the sum of the two themes does not dip in the middle
                let volume = level(Cue::Title) * (self.title * std::f32::consts::FRAC_PI_2).sin();
                self.orders.push(Order::Volume(Cue::Title, volume));
            }
        }
        let music = level(Cue::Theme) * gain(self.duck) * self.fade;
        if !self.over && music != self.music {
            self.music = music;
            self.orders.push(Order::Volume(Cue::Theme, music));
        }
    }

    /// A new game: silence whatever the old one left ringing, then the theme from its first bar.
    /// Started from the title screen, the title theme is not cut: it fades out under the
    /// game's theme in `TITLE_FADE`.
    pub fn restart(&mut self) {
        let from_title = self.title > 0.0;
        let mut orders = std::mem::take(&mut self.orders);
        orders.clear();
        orders.extend(
            Cue::ALL
                .into_iter()
                .filter(|&cue| !(from_title && cue == Cue::Title))
                .map(Order::Stop),
        );
        *self = Desk::new();
        if from_title {
            self.title = 1.0;
            self.title_fading = true;
        }
        orders.append(&mut self.orders);
        self.orders = orders;
        self.play(Cue::NewGame, 1.0);
    }
}

/// The audio device, with every cue loaded. Not tested: it needs a sound card.
///
/// A cue that fails to load is left out with a warning, and the game plays on without it: sound
/// only listens, so nothing in the game may stop for it. `muted` silences everything at once;
/// the last volume ordered for each cue is kept, so unmuting puts the mix back as it was.
pub struct Speakers {
    sounds: Vec<Option<Sound>>, // indexed by `Cue as usize`
    volumes: Vec<f32>,          // the last volume ordered for each cue
    muted: bool,
}

impl Speakers {
    pub async fn load() -> Self {
        let mut sounds = Vec::with_capacity(Cue::ALL.len());
        for cue in Cue::ALL {
            match load_sound_from_bytes(cue.bytes()).await {
                Ok(sound) => sounds.push(Some(sound)),
                Err(e) => {
                    eprintln!("polyfall: no sound for {cue:?}: {e:?}");
                    sounds.push(None);
                }
            }
        }
        Speakers {
            sounds,
            volumes: vec![0.0; Cue::ALL.len()],
            muted: false,
        }
    }

    pub fn obey(&mut self, orders: impl Iterator<Item = Order>) {
        for order in orders {
            match order {
                Order::Play(cue, volume) => self.start(cue, volume, false),
                Order::Loop(cue, volume) => self.start(cue, volume, true),
                Order::Volume(cue, volume) => {
                    self.volumes[cue as usize] = volume;
                    if let Some(sound) = &self.sounds[cue as usize] {
                        set_sound_volume(sound, self.heard(volume));
                    }
                }
                Order::Stop(cue) => {
                    if let Some(sound) = &self.sounds[cue as usize] {
                        stop_sound(sound);
                    }
                }
            }
        }
    }

    /// Mutes or unmutes every sound, including those already playing.
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        for cue in Cue::ALL {
            if let Some(sound) = &self.sounds[cue as usize] {
                set_sound_volume(sound, self.heard(self.volumes[cue as usize]));
            }
        }
    }

    pub fn muted(&self) -> bool {
        self.muted
    }

    fn heard(&self, volume: f32) -> f32 {
        if self.muted {
            0.0
        } else {
            volume
        }
    }

    fn start(&mut self, cue: Cue, volume: f32, looped: bool) {
        self.volumes[cue as usize] = volume;
        if let Some(sound) = &self.sounds[cue as usize] {
            let volume = self.heard(volume);
            play_sound(sound, PlaySoundParams { looped, volume });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{H, W};

    const FRAME: f32 = 1.0 / 60.0;

    /// A desk in the middle of a game: the order that starts the theme already taken.
    fn desk() -> Desk {
        let mut d = Desk::new();
        d.orders().for_each(drop);
        d
    }

    fn taken(d: &mut Desk) -> Vec<Order> {
        d.orders().collect()
    }

    /// The cues played once, in order, without their volumes.
    fn played(orders: &[Order]) -> Vec<Cue> {
        orders
            .iter()
            .filter_map(|o| match o {
                Order::Play(cue, _) => Some(*cue),
                _ => None,
            })
            .collect()
    }

    fn volume_of(orders: &[Order], cue: Cue) -> f32 {
        orders
            .iter()
            .find_map(|o| match o {
                Order::Play(c, v) if *c == cue => Some(*v),
                _ => None,
            })
            .unwrap()
    }

    /// A lock of `cells` cells that clears `rows` rows, as the game reports it.
    fn locked(cells: usize, rows: usize) -> Event {
        Event::Locked {
            cells: (0..cells as i32).map(|i| (0, H - 1 - i)).collect(),
            color: 1,
            cleared: (H as usize - rows..H as usize).collect(),
            before: Box::new([[0; W as usize]; H as usize]),
            points: (rows * rows) as u32 * 100,
        }
    }

    /// The last theme volume the desk ordered, after running it for `seconds`.
    fn theme_after(d: &mut Desk, paused: bool, seconds: f32) -> Vec<f32> {
        let mut volumes = Vec::new();
        let mut t = 0.0;
        while t < seconds {
            d.frame(paused, FRAME);
            t += FRAME;
            for o in taken(d) {
                if let Order::Volume(Cue::Theme, v) = o {
                    volumes.push(v);
                }
            }
        }
        volumes
    }

    #[test]
    fn the_theme_starts_with_the_desk() {
        let mut d = Desk::new();
        assert_eq!(
            taken(&mut d),
            vec![Order::Loop(Cue::Theme, level(Cue::Theme))]
        );
        // and an idle frame has nothing to add
        d.frame(false, FRAME);
        assert!(taken(&mut d).is_empty());
    }

    #[test]
    fn every_action_of_the_player_is_answered() {
        let mut d = desk();
        for (event, cue) in [
            (Event::Moved { dx: -1, dy: 0 }, Cue::MoveLeft),
            (Event::Moved { dx: 1, dy: 0 }, Cue::MoveRight),
            (Event::Rotated { dx: 0, dy: 0 }, Cue::Rotate),
            (Event::Rotated { dx: 2, dy: 0 }, Cue::Rotate), // a kicked turn is still a turn
            (Event::Blocked, Cue::Blocked),
            (locked(4, 0), Cue::Lock),
        ] {
            d.on(&event);
            assert_eq!(played(&taken(&mut d)), vec![cue], "{event:?}");
            d.frame(false, 0.2);
            taken(&mut d);
        }
        d.on(&Event::SoftDropped);
        assert_eq!(
            taken(&mut d),
            vec![Order::Loop(Cue::SoftDrop, level(Cue::SoftDrop))]
        );
    }

    #[test]
    fn gravity_makes_no_sound() {
        let mut d = desk();
        for _ in 0..20 {
            d.on(&Event::Moved { dx: 0, dy: 1 });
            d.frame(false, 0.6);
        }
        assert!(taken(&mut d).is_empty());
    }

    #[test]
    fn a_held_key_ticks_no_faster_than_the_mixer_can_play() {
        // a press, then the key repeating every 50 ms from 170 ms on, as `Game::repeats` does
        let mut d = desk();
        let (mut heard, mut t, mut next_repeat) = (Vec::new(), 0.0, 0.17);
        d.on(&Event::Moved { dx: 1, dy: 0 });
        while t < 1.0 {
            for o in taken(&mut d) {
                heard.push((t, o));
            }
            d.frame(false, FRAME);
            t += FRAME;
            if t >= next_repeat {
                next_repeat += 0.05;
                d.on(&Event::Moved { dx: 1, dy: 0 });
            }
        }
        let times: Vec<f32> = heard.iter().map(|(t, _)| *t).collect();
        assert!(times.windows(2).all(|w| w[1] - w[0] >= MOVE_GAP));
        // 17 moves were made; about every other one is heard
        assert!((8..=10).contains(&heard.len()), "{}", heard.len());
        // the press is at full level; once the key is repeating, the ticks are quieter
        let full = level(Cue::MoveRight);
        assert_eq!(heard[0].1, Order::Play(Cue::MoveRight, full));
        assert_eq!(
            heard.last().unwrap().1,
            Order::Play(Cue::MoveRight, full * REPEAT_LEVEL)
        );

        // two separate presses, a fifth of a second apart, are both at full level
        let mut d = desk();
        d.on(&Event::Moved { dx: -1, dy: 0 });
        d.frame(false, 0.2);
        d.on(&Event::Moved { dx: -1, dy: 0 });
        let full = Order::Play(Cue::MoveLeft, level(Cue::MoveLeft));
        assert_eq!(taken(&mut d), vec![full, full]);
    }

    #[test]
    fn the_soft_drop_runs_while_rows_fall_and_stops_when_they_stop() {
        let mut d = desk();
        let mut orders = Vec::new();
        for _ in 0..10 {
            d.on(&Event::Moved { dx: 0, dy: 1 });
            d.on(&Event::SoftDropped);
            for _ in 0..3 {
                d.frame(false, FRAME); // a row every three frames
            }
            orders.extend(taken(&mut d));
        }
        // ten rows, one loop, and it has not been stopped
        assert_eq!(
            orders,
            vec![Order::Loop(Cue::SoftDrop, level(Cue::SoftDrop))]
        );
        // a frame that takes a third of a second does not stop it, if a row fell in that frame
        d.on(&Event::SoftDropped);
        d.frame(false, 0.33);
        assert!(taken(&mut d).is_empty());
        // Down is let go: the run stops by itself, once
        for _ in 0..12 {
            d.frame(false, FRAME);
        }
        assert_eq!(taken(&mut d), vec![Order::Stop(Cue::SoftDrop)]);

        // a piece that lands stops it at once, before its own sound
        d.on(&Event::SoftDropped);
        d.on(&locked(3, 0));
        let orders = taken(&mut d);
        assert_eq!(orders[1], Order::Stop(Cue::SoftDrop));
        assert_eq!(played(&orders), vec![Cue::Lock]);
        // and the next soft drop starts it again
        d.on(&Event::SoftDropped);
        assert_eq!(taken(&mut d).len(), 1);
    }

    #[test]
    fn a_longer_fall_lands_harder_and_a_settled_piece_has_no_thump() {
        let mut d = desk();
        d.on(&locked(4, 0));
        assert_eq!(played(&taken(&mut d)), vec![Cue::Lock]);

        let mut last = 0.0;
        for rows in [0, 1, 4, 10] {
            d.on(&Event::HardDropped { rows });
            d.on(&locked(4, 0));
            let orders = taken(&mut d);
            assert_eq!(played(&orders), vec![Cue::Lock, Cue::HardDrop]);
            let thump = volume_of(&orders, Cue::HardDrop);
            assert!(thump > last, "{rows} rows");
            last = thump;
        }
        assert_eq!(last, level(Cue::HardDrop));
        // past ten rows it is as hard as it gets
        d.on(&Event::HardDropped { rows: 21 });
        d.on(&locked(4, 0));
        assert_eq!(volume_of(&taken(&mut d), Cue::HardDrop), last);
        // the hard drop was used up: the next lock is an ordinary one
        d.on(&locked(4, 0));
        assert_eq!(played(&taken(&mut d)), vec![Cue::Lock]);
    }

    #[test]
    fn a_bigger_piece_lands_heavier() {
        let mut d = desk();
        let mut last = 0.0;
        for cells in 3..=5 {
            d.on(&locked(cells, 0));
            let lock = volume_of(&taken(&mut d), Cue::Lock);
            assert!(lock > last);
            last = lock;
        }
    }

    #[test]
    fn each_count_of_rows_has_its_own_cue_and_ducks_the_theme_more() {
        let mut deepest = level(Cue::Theme);
        for rows in 1..=5 {
            let mut d = desk();
            d.on(&locked(5, rows));
            assert_eq!(
                played(&taken(&mut d)),
                vec![Cue::Lock, Cue::CLEARS[rows - 1]]
            );
            let volumes = theme_after(&mut d, false, 3.0);
            if rows == 1 {
                assert!(volumes.is_empty()); // one row leaves the theme alone
                continue;
            }
            // down at once, held, then back up without ever dipping again, and exactly home
            let low = volumes[0];
            assert!(low < deepest, "{rows} rows");
            deepest = low;
            assert!(volumes.windows(2).all(|w| w[1] >= w[0]));
            assert_eq!(*volumes.last().unwrap(), level(Cue::Theme));
        }
    }

    #[test]
    fn a_level_up_plays_its_cue_and_holds_the_theme_down_longer_than_a_clear() {
        // the cues played, and how long after them the theme is back at its own level
        let back_after = |events: &[Event]| {
            let mut d = desk();
            events.iter().for_each(|e| d.on(e));
            let cues = played(&taken(&mut d));
            let (mut t, mut back, mut last) = (0.0, 0.0, 0.0);
            while t < 6.0 {
                d.frame(false, FRAME);
                t += FRAME;
                for o in taken(&mut d) {
                    if let Order::Volume(Cue::Theme, v) = o {
                        (back, last) = (t, v);
                    }
                }
            }
            assert_eq!(last, level(Cue::Theme));
            (cues, back)
        };
        let (cues, clear) = back_after(&[locked(4, 4)]);
        assert_eq!(cues, vec![Cue::Lock, Cue::Clear4]);
        let (cues, alone) = back_after(&[Event::LevelUp { level: 1 }]);
        assert_eq!(cues, vec![Cue::LevelUp]);
        let (cues, both) = back_after(&[locked(4, 4), Event::LevelUp { level: 1 }]);
        assert_eq!(cues, vec![Cue::Lock, Cue::Clear4, Cue::LevelUp]);
        // the level-up holds the theme down for the length of its chord; with a clear in the
        // same lock, the deeper of the two ducks is held for the longer of the two times
        assert!(alone > clear && both > alone, "{clear} {alone} {both}");
    }

    #[test]
    fn a_pause_cuts_what_was_ringing_and_fades_the_theme_out_and_back() {
        let mut d = desk();
        d.on(&Event::SoftDropped);
        d.on(&locked(4, 4));
        // one frame of play: the four rows have lowered the theme
        let lowered = theme_after(&mut d, false, FRAME)[0];
        assert!(lowered < level(Cue::Theme));

        d.frame(true, FRAME);
        let orders = taken(&mut d);
        // every cue but the theme is stopped, then the pause is heard, once
        let stops = orders
            .iter()
            .filter(|o| matches!(o, Order::Stop(_)))
            .count();
        assert_eq!(stops, Cue::ALL.len() - 1);
        assert!(!orders.contains(&Order::Stop(Cue::Theme)));
        assert_eq!(played(&orders), vec![Cue::Pause]);
        // from where the clear had put it, the theme goes down to nothing, never back up, in
        // about FADE_OUT
        let mut down = vec![lowered];
        down.extend(orders.iter().filter_map(|o| match o {
            Order::Volume(Cue::Theme, v) => Some(*v),
            _ => None,
        }));
        down.extend(theme_after(&mut d, true, 1.0));
        assert!(down.windows(2).all(|w| w[1] <= w[0]), "{down:?}");
        assert_eq!(*down.last().unwrap(), 0.0);
        assert!((down.len() - 1) as f32 * FRAME <= FADE_OUT + 2.0 * FRAME);
        // a long pause orders nothing more
        d.frame(true, 30.0);
        assert!(taken(&mut d).is_empty());

        d.frame(false, FRAME);
        let orders = taken(&mut d);
        assert_eq!(played(&orders), vec![Cue::Resume]);
        let up = theme_after(&mut d, false, 1.0);
        assert!(up.windows(2).all(|w| w[1] >= w[0]));
        assert_eq!(*up.last().unwrap(), level(Cue::Theme));
        // it came back in FADE_IN, not held down by the duck of a clear the pause had cut
        assert!(up.len() as f32 * FRAME <= FADE_IN + 2.0 * FRAME);
        // the soft drop was cut by the pause too: the next row starts it again
        d.on(&Event::SoftDropped);
        assert_eq!(taken(&mut d).len(), 1);
    }

    #[test]
    fn the_end_stops_the_theme_and_nothing_brings_it_back() {
        let mut d = desk();
        d.on(&locked(4, 2));
        d.on(&Event::LevelUp { level: 1 });
        d.on(&Event::GameOver);
        let orders = taken(&mut d);
        assert_eq!(
            played(&orders),
            vec![Cue::Lock, Cue::Clear2, Cue::LevelUp, Cue::GameOver]
        );
        // the level-up from the same lock is stopped again before it is heard
        let stop = |cue| orders.iter().position(|o| *o == Order::Stop(cue)).unwrap();
        let level_up = orders
            .iter()
            .position(|o| matches!(o, Order::Play(Cue::LevelUp, _)))
            .unwrap();
        assert!(stop(Cue::LevelUp) > level_up);
        assert!(orders.contains(&Order::Stop(Cue::Theme)));
        // the duck those rows started must not reach a theme that is no longer playing
        assert!(theme_after(&mut d, false, 5.0).is_empty());
    }

    /// The title theme's volume after each frame for `seconds`, and whether it was stopped.
    fn title_after(d: &mut Desk, seconds: f32) -> (Vec<f32>, bool) {
        let (mut volumes, mut stopped, mut t) = (Vec::new(), false, 0.0);
        while t < seconds {
            d.frame(false, FRAME);
            t += FRAME;
            for o in taken(d) {
                match o {
                    Order::Volume(Cue::Title, v) => volumes.push(v),
                    Order::Stop(Cue::Title) => stopped = true,
                    _ => {}
                }
            }
        }
        (volumes, stopped)
    }

    #[test]
    fn the_title_screen_plays_the_title_theme_and_nothing_else() {
        let mut d = Desk::title();
        assert_eq!(
            taken(&mut d),
            vec![Order::Loop(Cue::Title, level(Cue::Title))]
        );
        // it holds steady for as long as the screen is up
        assert_eq!(title_after(&mut d, 30.0), (vec![], false));
    }

    #[test]
    fn a_game_started_from_the_title_fades_the_title_theme_out_under_the_game_theme() {
        let mut d = Desk::title();
        taken(&mut d);
        d.restart();
        let orders = taken(&mut d);
        assert!(!orders.contains(&Order::Stop(Cue::Title)));
        assert!(orders.contains(&Order::Loop(Cue::Theme, level(Cue::Theme))));
        assert!(orders.contains(&Order::Play(Cue::NewGame, level(Cue::NewGame))));
        let (volumes, stopped) = title_after(&mut d, TITLE_FADE + 0.2);
        assert!(stopped);
        assert!(volumes.windows(2).all(|w| w[1] < w[0]));
        assert!(volumes[0] < level(Cue::Title));
        // a second restart, mid-game, has no title theme left to fade
        d.restart();
        assert!(taken(&mut d).contains(&Order::Stop(Cue::Title)));
        assert_eq!(title_after(&mut d, 2.0), (vec![], false));
    }

    #[test]
    fn back_to_the_title_stops_the_game_and_starts_the_title_theme() {
        let mut d = desk();
        d.on(&locked(4, 4));
        d.back_to_title();
        let orders = taken(&mut d);
        let n = Cue::ALL.len();
        assert_eq!(orders[..n], Cue::ALL.map(Order::Stop));
        assert_eq!(orders[n..], [Order::Loop(Cue::Title, level(Cue::Title))]);
        // the duck of the cut clear is gone with the game
        d.frame(false, 2.0);
        assert!(taken(&mut d).is_empty());
    }

    #[test]
    fn a_new_game_silences_the_old_one_and_starts_the_theme_again() {
        let mut d = desk();
        d.on(&Event::SoftDropped);
        d.on(&locked(4, 4));
        d.on(&Event::GameOver);
        d.frame(false, FRAME);
        d.restart();
        let orders = taken(&mut d);
        let n = Cue::ALL.len();
        assert_eq!(orders[..n], Cue::ALL.map(Order::Stop));
        assert_eq!(
            orders[n..],
            [
                Order::Loop(Cue::Theme, level(Cue::Theme)),
                Order::Play(Cue::NewGame, level(Cue::NewGame)),
            ]
        );
        // nothing of the old game is left: no duck, no ending, no run
        assert!(theme_after(&mut d, false, 2.0).is_empty());
        d.on(&Event::SoftDropped);
        assert_eq!(taken(&mut d).len(), 1);
        d.on(&locked(4, 2));
        d.frame(false, FRAME);
        assert!(taken(&mut d)
            .iter()
            .any(|o| matches!(o, Order::Volume(Cue::Theme, _))));
    }

    #[test]
    fn every_cue_has_a_file_and_a_level_that_cannot_clip_alone() {
        for cue in Cue::ALL {
            assert!(cue.bytes().len() > 44, "{cue:?}");
            assert!(level(cue) > 0.0 && level(cue) <= 1.0, "{cue:?}");
        }
        // the index `Speakers` uses is the position in `ALL`
        for (i, cue) in Cue::ALL.iter().enumerate() {
            assert_eq!(*cue as usize, i);
        }
    }
}
