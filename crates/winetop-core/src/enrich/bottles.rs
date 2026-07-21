use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::debug;

#[derive(Debug, Default)]
pub struct BottlesIndex {
    by_prefix: HashMap<String, String>,
    /// Bottle names that claim to be running but may be sandboxed.
    opaque_running: Vec<String>,
}

impl BottlesIndex {
    pub fn load() -> Self {
        let mut by_prefix = HashMap::new();
        let mut opaque_running = Vec::new();
        let Some(home) = dirs::home_dir() else {
            return Self::default();
        };
        let bottle_roots = [
            home.join(".local/share/bottles/bottles"),
            home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
        ];
        for root in bottle_roots {
            load_bottles_dir(&root, &mut by_prefix, &mut opaque_running);
        }
        debug!(
            count = by_prefix.len(),
            opaque = opaque_running.len(),
            "loaded bottles index"
        );
        Self {
            by_prefix,
            opaque_running,
        }
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

    pub fn opaque_hints(&self) -> &[String] {
        &self.opaque_running
    }
}

fn load_bottles_dir(
    root: &Path,
    by_prefix: &mut HashMap<String, String>,
    opaque: &mut Vec<String>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "bottle".into());
        by_prefix.insert(path.display().to_string(), name.clone());

        // bottle.yml / yml may contain Session_arguments / running hints
        for yml in ["bottle.yml", "bottle.yaml"] {
            let yml_path = path.join(yml);
            if let Ok(text) = fs::read_to_string(&yml_path) {
                if text.contains("Running: true") || text.contains("running: true") {
                    opaque.push(name.clone());
                }
                if let Some(n) = yaml_key(&text, "Name").or_else(|| yaml_key(&text, "name")) {
                    by_prefix.insert(path.display().to_string(), n);
                }
            }
        }
    }
}

fn yaml_key(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix(&format!("{key}:")) {
            return Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}
