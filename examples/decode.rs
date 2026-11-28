//! Decodes sound files exactly as the game's mixer will: with `audrey`, the crate macroquad's
//! `quad-snd` loads a sound with, at the same version and with the same features.
//!
//! The sound bench (`tools/sound`) measures what this writes, so a file is judged by what the
//! game will play and not by what another decoder makes of it. They differ: this one pads the
//! end of a short Ogg file, which matters for anything that loops.
//!
//! usage: cargo run --release --example decode -- <out dir> <file>...
//!
//! For each file it writes `<out dir>/<file stem>.f32`, interleaved stereo as little-endian
//! 32-bit floats (a mono file is doubled, as the mixer does), and prints one line:
//! `<stem> <channels> <rate> <frames>`.

use std::io::Write;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [out, files @ ..] = args.as_slice() else {
        eprintln!("usage: decode <out dir> <file>...");
        std::process::exit(2);
    };
    for file in files {
        let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{file}: {e}"));
        let mut reader = audrey::Reader::new(std::io::Cursor::new(bytes))
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        let description = reader.description();
        let channels = description.channel_count();
        let samples: Vec<f32> = reader.samples::<f32>().map(Result::unwrap).collect();
        let stereo: Vec<f32> = match channels {
            1 => samples.iter().flat_map(|&s| [s, s]).collect(),
            2 => samples,
            n => panic!("{file}: {n} channels; the mixer takes one or two"),
        };
        let stem = Path::new(file).file_stem().unwrap().to_string_lossy();
        let mut bytes = Vec::with_capacity(stereo.len() * 4);
        for s in &stereo {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::File::create(Path::new(out).join(format!("{stem}.f32")))
            .and_then(|mut f| f.write_all(&bytes))
            .unwrap_or_else(|e| panic!("{out}: {e}"));
        println!(
            "{stem} {channels} {} {}",
            description.sample_rate(),
            stereo.len() / 2
        );
    }
}
