use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::debug;

#[derive(Debug, Default)]
pub struct HeroicIndex {
    by_prefix: HashMap<String, String>,
}

impl HeroicIndex {
    pub fn load() -> Self {
        let mut by_prefix = HashMap::new();
        let Some(home) = dirs::home_dir() else {
            return Self { by_prefix };
        };
        let roots = [
            home.join(".config/heroic/GamesConfig"),
            home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic/GamesConfig"),
        ];
        for root in roots {
            load_games_config(&root, &mut by_prefix);
        }
        // Also scan installed.json style files
        for path in [
            home.join(".config/heroic/legendaryConfig/legendary/installed.json"),
            home.join(".config/heroic/gog_store/installed.json"),
            home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic/legendaryConfig/legendary/installed.json"),
        ] {
            load_installed_json(&path, &mut by_prefix);
        }
        debug!(count = by_prefix.len(), "loaded heroic names");
        Self { by_prefix }
    }

    pub fn name_for_prefix(&self, prefix: &Path) -> Option<String> {
        let key = prefix.display().to_string();
        self.by_prefix.get(&key).cloned().or_else(|| {
            self.by_prefix.iter().find_map(|(k, v)| {
                if key.starts_with(k) || k.starts_with(&key) {
                    Some(v.clone())
                } else {
                    None
                }
            })
        })
    }
}

fn load_games_config(dir: &Path, out: &mut HashMap<String, String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        // Very light JSON key scan without full serde schema
        let title = find_json_string(&text, "title")
            .or_else(|| find_json_string(&text, "app_name"))
            .or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()));
        let wine_prefix = find_json_string(&text, "winePrefix")
            .or_else(|| find_json_string(&text, "wine_prefix"));
        if let (Some(t), Some(p)) = (title, wine_prefix) {
            out.insert(p, t);
        }
    }
}

fn load_installed_json(path: &Path, out: &mut HashMap<String, String>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    // Best-effort: pair "title" near "winePrefix" is hard; store install paths as names
    if let Some(title) = find_json_string(&text, "title") {
        if let Some(prefix) = find_json_string(&text, "winePrefix") {
            out.insert(prefix, title);
        }
    }
}

fn find_json_string(text: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let idx = text.find(&pattern)?;
    let after = &text[idx + pattern.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
