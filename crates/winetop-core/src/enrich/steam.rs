use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Default)]
pub struct SteamIndex {
    names: HashMap<u32, String>,
}

impl SteamIndex {
    pub fn load() -> Self {
        let mut names = HashMap::new();
        for lib in steam_library_roots() {
            let steamapps = lib.join("steamapps");
            let Ok(entries) = fs::read_dir(&steamapps) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !fname.starts_with("appmanifest_") || !fname.ends_with(".acf") {
                    continue;
                }
                if let Some((id, name)) = parse_appmanifest(&path) {
                    names.insert(id, name);
                }
            }
        }
        debug!(count = names.len(), "loaded steam app names");
        Self { names }
    }

    pub fn name_for(&self, appid: u32) -> Option<String> {
        self.names.get(&appid).cloned()
    }
}

fn steam_library_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let Some(home) = dirs::home_dir() else {
        return roots;
    };

    let candidates = [
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
    ];

    for c in candidates {
        if c.is_dir() {
            roots.push(c.clone());
            roots.extend(parse_libraryfolders(
                &c.join("steamapps/libraryfolders.vdf"),
            ));
            roots.extend(parse_libraryfolders(&c.join("config/libraryfolders.vdf")));
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn parse_libraryfolders(path: &Path) -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.contains("\"path\"") {
            if let Some(path) = extract_vdf_string_value(line) {
                let p = PathBuf::from(path);
                if p.is_dir() {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn parse_appmanifest(path: &Path) -> Option<(u32, String)> {
    let text = fs::read_to_string(path).ok()?;
    let mut appid = None;
    let mut name = None;
    for line in text.lines() {
        let line = line.trim();
        if line.contains("\"appid\"") {
            appid = extract_vdf_string_value(line).and_then(|s| s.parse().ok());
        }
        if line.contains("\"name\"") {
            name = extract_vdf_string_value(line);
        }
    }
    Some((appid?, name?))
}

fn extract_vdf_string_value(line: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut in_q = false;
    let mut cur = String::new();
    for c in line.chars() {
        if c == '"' {
            if in_q {
                parts.push(std::mem::take(&mut cur));
                in_q = false;
            } else {
                in_q = true;
            }
        } else if in_q {
            cur.push(c);
        }
    }
    if parts.len() >= 2 {
        Some(parts[1].clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdf_value() {
        assert_eq!(
            extract_vdf_string_value("\"path\"\t\t\"/mnt/games\"").as_deref(),
            Some("/mnt/games")
        );
    }
}
