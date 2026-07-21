use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Default)]
pub struct LutrisIndex {
    /// normalized prefix path -> game name
    by_prefix: HashMap<String, String>,
}

impl LutrisIndex {
    pub fn load() -> Self {
        let mut by_prefix = HashMap::new();
        load_from_yml(&mut by_prefix);
        load_from_pga(&mut by_prefix);
        debug!(count = by_prefix.len(), "loaded lutris game names");
        Self { by_prefix }
    }

    pub fn name_for_prefix(&self, prefix: &Path) -> Option<String> {
        let key = normalize(prefix);
        self.by_prefix.get(&key).cloned().or_else(|| {
            // try without trailing slash variants
            self.by_prefix.iter().find_map(|(k, v)| {
                if k.ends_with(&key) || key.ends_with(k) {
                    Some(v.clone())
                } else {
                    None
                }
            })
        })
    }
}

fn normalize(p: &Path) -> String {
    let s = p.display().to_string();
    if let Ok(canon) = fs::canonicalize(p) {
        return canon.display().to_string();
    }
    s
}

fn load_from_yml(out: &mut HashMap<String, String>) {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let dirs = [
        home.join(".config/lutris/games"),
        home.join(".var/app/net.lutris.Lutris/config/lutris/games"),
    ];
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yml")
                && path.extension().and_then(|e| e.to_str()) != Some("yaml")
            {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let mut name = None;
            let mut prefix = None;
            for line in text.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("name:") {
                    name = Some(unquote(rest.trim()));
                }
                if let Some(rest) = t.strip_prefix("prefix:") {
                    let p = unquote(rest.trim());
                    prefix = Some(expand(&p));
                }
                // game section often nests prefix under wine
                if let Some(rest) = t.strip_prefix("prefix:") {
                    let p = unquote(rest.trim());
                    prefix = Some(expand(&p));
                }
            }
            if let (Some(n), Some(p)) = (name, prefix) {
                out.insert(normalize(&p), n);
            }
        }
    }
}

fn load_from_pga(out: &mut HashMap<String, String>) {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let db_paths = [
        home.join(".local/share/lutris/pga.db"),
        home.join(".var/app/net.lutris.Lutris/data/lutris/pga.db"),
    ];
    for db_path in db_paths {
        if !db_path.is_file() {
            continue;
        }
        if let Err(e) = load_pga_db(&db_path, out) {
            debug!(error = %e, path = %db_path.display(), "lutris pga.db read failed");
        }
    }
}

fn load_pga_db(path: &Path, out: &mut HashMap<String, String>) -> Result<(), String> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| e.to_string())?;

    // Schema varies; try common columns.
    let queries = [
        "SELECT name, directory FROM games WHERE directory IS NOT NULL",
        "SELECT name, prefix FROM games WHERE prefix IS NOT NULL",
    ];
    for q in queries {
        let Ok(mut stmt) = conn.prepare(q) else {
            continue;
        };
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let dir: String = row.get(1)?;
            Ok((name, dir))
        });
        let Ok(rows) = rows else {
            continue;
        };
        for row in rows.flatten() {
            let (name, dir) = row;
            let prefix = expand(&dir);
            // directory may be game install dir — also try prefix subdir
            out.insert(normalize(&prefix), name.clone());
            let nested = prefix.join("prefix");
            if nested.is_dir() {
                out.insert(normalize(&nested), name.clone());
            }
            let pfx = prefix.join("pfx");
            if pfx.is_dir() {
                out.insert(normalize(&pfx), name);
            }
        }
    }
    Ok(())
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_string()
}

fn expand(s: &str) -> PathBuf {
    crate::util::expand_home(s)
}
