use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// 阅读位置：兼容旧版（整书行号）和新版（章节 + 章内行号）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Position {
    Line(usize),
    Chapter { chapter: usize, offset: usize },
}

impl Position {
    /// 旧版行号属于整本书，折行后才能映射到新版本的章节位置。
    pub fn resolve(self, chapter_lengths: &[usize]) -> (usize, usize) {
        if chapter_lengths.is_empty() {
            return (0, 0);
        }
        match self {
            Position::Line(mut offset) => {
                for (chapter, &length) in chapter_lengths.iter().enumerate() {
                    if offset < length || chapter + 1 == chapter_lengths.len() {
                        return (chapter, offset.min(length.saturating_sub(1)));
                    }
                    offset -= length;
                }
                unreachable!()
            }
            Position::Chapter { chapter, offset } => {
                (chapter.min(chapter_lengths.len() - 1), offset)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, Progress};

    #[test]
    fn legacy_progress_restores_a_later_chapter() {
        let progress: Progress = toml::from_str("[positions]\nbook = 70\n").unwrap();
        assert_eq!(progress.get("book").resolve(&[61, 61]), (1, 9));
        assert_eq!(Position::Line(61).resolve(&[61, 61]), (1, 0));
    }

    #[test]
    fn chapter_progress_round_trips_alongside_legacy_entries() {
        let mut progress: Progress = toml::from_str("[positions]\nold = 70\n").unwrap();
        progress.set("new".to_string(), 1, 23);
        let restored: Progress = toml::from_str(&toml::to_string(&progress).unwrap()).unwrap();
        assert_eq!(restored.get("old").resolve(&[61, 61]), (1, 9));
        assert_eq!(restored.get("new").resolve(&[61, 61]), (1, 23));
    }

    #[test]
    fn progress_handles_shortened_or_empty_books() {
        assert_eq!(Position::Line(999).resolve(&[20, 30]), (1, 29));
        assert_eq!(Position::Line(10).resolve(&[]), (0, 0));
        assert_eq!(
            Position::Chapter {
                chapter: 9,
                offset: 10
            }
            .resolve(&[20]),
            (0, 10)
        );
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    pub positions: HashMap<String, Position>,
}

impl Progress {
    pub fn load() -> Self {
        let path = crate::config::progress_path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| toml::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = crate::config::progress_path();
        if let Ok(raw) = toml::to_string_pretty(self) {
            let _ = fs::write(path, raw);
        }
    }

    pub fn get(&self, key: &str) -> Position {
        self.positions
            .get(key)
            .copied()
            .unwrap_or(Position::Line(0))
    }

    pub fn set(&mut self, key: String, chapter: usize, offset: usize) {
        self.positions
            .insert(key, Position::Chapter { chapter, offset });
    }
}
