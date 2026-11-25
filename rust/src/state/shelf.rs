//! File shelf — remembers file locations so files "stay where they are".
//! Persists a list of file paths the user has interacted with.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Shelf {
    pub files: Vec<String>,
}

impl Shelf {
    pub fn path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("MacBookNotch").join("shelf.json")
    }

    pub fn load() -> Self {
        match fs::read_to_string(Self::path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        if let Some(parent) = Self::path().parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("warning: could not create shelf dir: {e}");
                return;
            }
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            if let Err(e) = fs::write(Self::path(), text) {
                eprintln!("warning: could not save shelf: {e}");
            }
        }
    }

    /// Add a file path (dedup, keep order, cap at 64).
    pub fn add(&mut self, path: &str) {
        if !self.files.iter().any(|f| f == path) {
            self.files.push(path.to_string());
            if self.files.len() > 64 {
                self.files.drain(0..self.files.len() - 64);
            }
        }
    }

    pub fn remove(&mut self, path: &str) {
        self.files.retain(|f| f != path);
    }
}
