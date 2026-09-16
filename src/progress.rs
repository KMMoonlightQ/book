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
    pub fn chapter(self) -> usize {
        match self {
            Position::Line(_) => 0,
            Position::Chapter { chapter, .. } => chapter,
        }
    }

    pub fn offset(self) -> usize {
        match self {
            Position::Line(offset) => offset,
            Position::Chapter { offset, .. } => offset,
        }
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
        self.positions.get(key).copied().unwrap_or(Position::Line(0))
    }

    pub fn set(&mut self, key: String, chapter: usize, offset: usize) {
        self.positions
            .insert(key, Position::Chapter { chapter, offset });
    }
}
