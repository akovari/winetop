use std::path::{Path, PathBuf};

/// Extract a Steam AppId from a compatdata path segment.
pub fn appid_from_compatdata_path(path: &Path) -> Option<u32> {
    let mut components: Vec<_> = path.components().collect();
    while let Some(c) = components.pop() {
        let s = c.as_os_str().to_string_lossy();
        if s == "pfx" || s == "prefix" {
            continue;
        }
        if let Some(prev) = components.last() {
            if prev.as_os_str() == "compatdata" {
                return s.parse().ok();
            }
        }
    }
    None
}

/// Parse `SteamLaunch AppId=<n>` from a cmdline string.
pub fn parse_steam_launch_appid(cmdline: &str) -> Option<u32> {
    const PREFIX: &str = "SteamLaunch AppId=";
    let idx = cmdline.find(PREFIX)?;
    let rest = &cmdline[idx + PREFIX.len()..];
    let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if id.is_empty() {
        return None;
    }
    // Avoid substring collisions (440 vs 4400): next char must be space or end.
    let after = rest.chars().nth(id.len());
    if after.is_some_and(|c| !c.is_whitespace() && c != '\0') {
        return None;
    }
    id.parse().ok()
}

/// Best-effort Windows image name from cmdline (last `.exe` token).
pub fn windows_image_from_cmdline(cmdline: &str) -> Option<String> {
    let lower = cmdline.to_ascii_lowercase();
    let mut best: Option<(usize, String)> = None;
    for (i, _) in lower.match_indices(".exe") {
        let start = cmdline[..i]
            .rfind(['/', '\\', ' ', '\t', '\0'])
            .map(|p| p + 1)
            .unwrap_or(0);
        let end = i + 4;
        let name = cmdline[start..end].to_string();
        if !name.is_empty() {
            best = Some((i, name));
        }
    }
    best.map(|(_, n)| n)
}

/// Differentiating detail for duplicate Windows images (path + args after `.exe`).
pub fn cmdline_detail(cmdline: &str) -> String {
    let lower = cmdline.to_ascii_lowercase();
    if let Some(exe_at) = lower.rmatch_indices(".exe").next().map(|(i, _)| i) {
        let path_start = cmdline[..exe_at]
            .rfind([' ', '\t'])
            .map(|p| p + 1)
            .unwrap_or(0);
        let path_end = exe_at + 4;
        let win_path = &cmdline[path_start..path_end];
        let args = cmdline[path_end..].trim();
        let short_path = shorten_win_path(win_path);
        if args.is_empty() {
            short_path
        } else {
            format!("{short_path} {args}")
        }
    } else {
        // Fall back: strip wine loader prefix, keep trailing tokens
        let trimmed = cmdline
            .split_whitespace()
            .filter(|t| {
                let t = t.to_ascii_lowercase();
                !t.ends_with("wine")
                    && !t.ends_with("wine64")
                    && !t.contains("preloader")
                    && !t.contains("wineserver")
            })
            .collect::<Vec<_>>()
            .join(" ");
        if trimmed.is_empty() {
            cmdline.to_string()
        } else {
            trimmed
        }
    }
}

fn shorten_win_path(path: &str) -> String {
    let norm = path.replace('\\', "/");
    let parts: Vec<&str> = norm.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() <= 2 {
        return path.to_string();
    }
    // Keep last two components: Battle.net/Battle.net.exe
    format!("…/{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
}

pub fn env_get(environ: &[(String, String)], key: &str) -> Option<String> {
    environ
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}

pub fn expand_home(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1}G", b / GB)
    } else if b >= MB {
        format!("{:.0}M", b / MB)
    } else if b >= KB {
        format!("{:.0}K", b / KB)
    } else {
        format!("{bytes}B")
    }
}

pub fn redact_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    upper.contains("TOKEN")
        || upper.contains("PASSWORD")
        || upper.contains("SECRET")
        || upper.contains("API_KEY")
        || upper.contains("AUTH")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_appid_anchored() {
        assert_eq!(
            parse_steam_launch_appid("reaper SteamLaunch AppId=440 --"),
            Some(440)
        );
        assert_eq!(
            parse_steam_launch_appid("reaper SteamLaunch AppId=4400 "),
            Some(4400)
        );
        // Should not match 440 inside 44012 when we check from AppId=44012
        assert_eq!(
            parse_steam_launch_appid("reaper SteamLaunch AppId=44012"),
            Some(44012)
        );
    }

    #[test]
    fn compatdata_appid() {
        let p = PathBuf::from("/mnt/games/SteamLibrary/steamapps/compatdata/1091500/pfx");
        assert_eq!(appid_from_compatdata_path(&p), Some(1091500));
    }

    #[test]
    fn windows_image() {
        assert_eq!(
            windows_image_from_cmdline("wine64 C:\\Games\\foo.exe --bar"),
            Some("foo.exe".into())
        );
    }

    #[test]
    fn cmdline_detail_args() {
        let d = cmdline_detail(
            "wine64-preloader C:\\Program Files (x86)\\Battle.net\\Battle.net.exe --type=gpu-process",
        );
        assert!(d.contains("Battle.net.exe"));
        assert!(d.contains("--type=gpu-process"));
    }
}
