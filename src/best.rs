//! The best score, kept between runs of the game in a one-line text file.
//!
//! One number and nothing else: no names, no table. A file that is missing or unreadable
//! counts as no best yet, and a failure to write is reported and otherwise ignored. Keeping
//! the score must never stop the game.

use std::path::{Path, PathBuf};

/// Where the best score lives: the user's data directory, by each platform's convention.
/// `None` when the environment names no home, as in some sandboxes; the best is then kept
/// only for the run.
pub fn file() -> Option<PathBuf> {
    let var = |name| {
        std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let dir = if cfg!(windows) {
        var("APPDATA")?
    } else if cfg!(target_os = "macos") {
        var("HOME")?.join("Library/Application Support")
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))?
    };
    Some(dir.join("polyfall").join("best"))
}

/// The best score in `path`, or 0.
pub fn load(path: Option<&Path>) -> u32 {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// Writes `best` to `path`, creating its directory.
pub fn save(path: Option<&Path>, best: u32) {
    let Some(path) = path else { return };
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(path, format!("{best}\n")));
    if let Err(e) = written {
        eprintln!(
            "polyfall: could not keep the best score in {}: {e}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("polyfall-best-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("nested").join("best")
    }

    #[test]
    fn a_saved_best_is_read_back() {
        let path = scratch("round");
        assert_eq!(load(Some(&path)), 0); // nothing yet
        save(Some(&path), 4200);
        assert_eq!(load(Some(&path)), 4200);
        save(Some(&path), 9);
        assert_eq!(load(Some(&path)), 9);
    }

    #[test]
    fn an_unreadable_file_counts_as_no_best() {
        let path = scratch("junk");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not a number").unwrap();
        assert_eq!(load(Some(&path)), 0);
        assert_eq!(load(None), 0);
        save(None, 5); // no file to keep it in: nothing happens
    }
}
