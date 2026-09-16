use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    pub positions: HashMap<String, usize>,
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

    pub fn get(&self, key: &str) -> usize {
        self.positions.get(key).copied().unwrap_or(0)
    }

    pub fn set(&mut self, key: String, offset: usize) {
        self.positions.insert(key, offset);
    }
}
