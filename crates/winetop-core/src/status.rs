//! Helpers for status-bar / scripting frontends (Waybar, etc.).

use crate::metrics::CpuTracker;
use crate::model::{Session, SessionSnapshot, Source};
use crate::{scan_with, Result};
use serde::Serialize;
use std::thread;
use std::time::Duration;

/// How to choose a single “current” session from a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickPolicy {
    /// Highest CPU% (then RSS as tie-break). Default for bars.
    #[default]
    Hottest,
    /// Highest RSS.
    Rss,
    /// Match focused window hints when possible; else hottest.
    Focused,
}

/// Optional focus hints from the compositor (Sway/wlroots, etc.).
#[derive(Debug, Clone, Default)]
pub struct FocusHint {
    pub app_id: Option<String>,
    pub class: Option<String>,
    pub title: Option<String>,
    pub steam_app_id: Option<u32>,
}

/// Filters applied before picking.
#[derive(Debug, Clone)]
pub struct StatusFilter {
    pub include_opaque: bool,
    pub min_rss_bytes: u64,
    pub steam_app_id: Option<u32>,
    pub session_id: Option<String>,
}

impl Default for StatusFilter {
    fn default() -> Self {
        Self {
            include_opaque: false,
            // Ignore tiny helper prefixes (launchers often leave small leftovers).
            min_rss_bytes: 64 * 1024 * 1024,
            steam_app_id: None,
            session_id: None,
        }
    }
}

/// Compact report for bars / JSON consumers.
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub present: bool,
    pub id: Option<String>,
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub source: Option<String>,
    pub steam_app_id: Option<u32>,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub process_count: usize,
    pub wineserver_alive: bool,
    pub session_count: usize,
}

impl StatusReport {
    pub fn empty(session_count: usize) -> Self {
        Self {
            present: false,
            id: None,
            name: None,
            short_name: None,
            source: None,
            steam_app_id: None,
            cpu_percent: 0.0,
            rss_bytes: 0,
            process_count: 0,
            wineserver_alive: false,
            session_count,
        }
    }

    pub fn from_session(session: &Session, session_count: usize) -> Self {
        Self {
            present: true,
            id: Some(session.id.clone()),
            name: Some(session.name.clone()),
            short_name: Some(shorten_name(&session.name, 22)),
            source: Some(session.source.as_str().to_string()),
            steam_app_id: session.steam_app_id,
            cpu_percent: session.cpu_percent,
            rss_bytes: session.rss_bytes,
            process_count: session.process_count(),
            wineserver_alive: session.wineserver_alive,
            session_count,
        }
    }
}

/// Two-phase scan so CPU% is meaningful for one-shot bar scripts.
///
/// `sample_ms == 0` skips the sleep (first sample still ≈0% CPU).
pub fn sample_snapshot(sample_ms: u64) -> Result<SessionSnapshot> {
    let mut tracker = CpuTracker::new();
    if sample_ms == 0 {
        return scan_with(&mut tracker);
    }
    let _ = scan_with(&mut tracker)?;
    thread::sleep(Duration::from_millis(sample_ms));
    scan_with(&mut tracker)
}

pub fn build_report(
    snap: &SessionSnapshot,
    policy: PickPolicy,
    filter: &StatusFilter,
    focus: Option<&FocusHint>,
) -> StatusReport {
    let candidates: Vec<&Session> = snap
        .sessions
        .iter()
        .filter(|s| filter_session(s, filter))
        .collect();
    let picked = pick_session(&candidates, policy, focus);
    match picked {
        Some(s) => StatusReport::from_session(s, snap.sessions.len()),
        None => StatusReport::empty(snap.sessions.len()),
    }
}

fn filter_session(s: &Session, filter: &StatusFilter) -> bool {
    if !filter.include_opaque && s.opaque {
        return false;
    }
    if s.rss_bytes < filter.min_rss_bytes {
        return false;
    }
    if let Some(id) = filter.steam_app_id {
        if s.steam_app_id != Some(id) {
            return false;
        }
    }
    if let Some(ref want) = filter.session_id {
        if &s.id != want {
            return false;
        }
    }
    true
}

pub fn pick_session<'a>(
    sessions: &[&'a Session],
    policy: PickPolicy,
    focus: Option<&FocusHint>,
) -> Option<&'a Session> {
    if sessions.is_empty() {
        return None;
    }
    match policy {
        PickPolicy::Hottest => sessions.iter().copied().max_by(|a, b| {
            a.cpu_percent
                .partial_cmp(&b.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.rss_bytes.cmp(&b.rss_bytes))
        }),
        PickPolicy::Rss => sessions
            .iter()
            .copied()
            .max_by(|a, b| a.rss_bytes.cmp(&b.rss_bytes)),
        PickPolicy::Focused => focus
            .and_then(|f| match_focus(sessions, f))
            .or_else(|| pick_session(sessions, PickPolicy::Hottest, None)),
    }
}

fn match_focus<'a>(sessions: &[&'a Session], focus: &FocusHint) -> Option<&'a Session> {
    if let Some(appid) = focus.steam_app_id {
        if let Some(s) = sessions.iter().find(|s| s.steam_app_id == Some(appid)) {
            return Some(*s);
        }
    }
    for key in [focus.app_id.as_deref(), focus.class.as_deref()]
        .into_iter()
        .flatten()
    {
        if let Some(appid) = parse_steam_app_class(key) {
            if let Some(s) = sessions.iter().find(|s| s.steam_app_id == Some(appid)) {
                return Some(*s);
            }
        }
    }
    let title = focus.title.as_deref()?.to_lowercase();
    if title.is_empty() {
        return None;
    }
    sessions
        .iter()
        .copied()
        .filter(|s| {
            let name = s.name.to_lowercase();
            !name.is_empty() && (title.contains(&name) || name.contains(&title))
        })
        .max_by(|a, b| a.name.len().cmp(&b.name.len()))
}

/// Parse `steam_app_12345` / `steam_proton_12345` style class/app_id.
pub fn parse_steam_app_class(s: &str) -> Option<u32> {
    let lower = s.to_ascii_lowercase();
    for prefix in ["steam_app_", "steam_proton_"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return digits.parse().ok();
            }
        }
    }
    None
}

pub fn shorten_name(name: &str, max_chars: usize) -> String {
    let count = name.chars().count();
    if count <= max_chars {
        return name.to_string();
    }
    let take = max_chars.saturating_sub(1);
    let mut out: String = name.chars().take(take).collect();
    out.push('…');
    out
}

pub fn source_icon(source: Source) -> &'static str {
    match source {
        Source::Steam => "󰓓",
        Source::Lutris => "󰊗",
        Source::Heroic => "󰮂",
        Source::Bottles => "󰡔",
        Source::Wine | Source::Unknown => "󰆍",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProcessKind, WineProcess};
    use std::path::PathBuf;

    fn session(name: &str, source: Source, cpu: f32, rss: u64, appid: Option<u32>) -> Session {
        Session {
            id: format!("id-{name}"),
            source,
            name: name.into(),
            prefix: Some(PathBuf::from("/tmp/pfx")),
            steam_app_id: appid,
            runner: None,
            processes: vec![WineProcess {
                pid: 1,
                ppid: 0,
                name: "game.exe".into(),
                windows_image: Some("game.exe".into()),
                cmdline: String::new(),
                exe_path: None,
                cwd: None,
                kind: ProcessKind::WindowsExe,
                state: "R".into(),
                cpu_percent: cpu,
                rss_bytes: rss,
                threads: 1,
                start_time_secs: 0,
                is_session_root: true,
                environ_keys: vec![],
                wine_prefix: None,
                steam_app_id: appid,
                steam_compat_data: None,
                lutris_uuid: None,
            }],
            cpu_percent: cpu,
            rss_bytes: rss,
            wineserver_alive: true,
            opaque: false,
            notes: vec![],
        }
    }

    #[test]
    fn hottest_wins() {
        let a = session("A", Source::Steam, 10.0, 1_000_000_000, Some(1));
        let b = session("B", Source::Steam, 80.0, 500_000_000, Some(2));
        let refs = vec![&a, &b];
        let picked = pick_session(&refs, PickPolicy::Hottest, None).unwrap();
        assert_eq!(picked.name, "B");
    }

    #[test]
    fn focused_steam_app_class() {
        let a = session("Civ", Source::Steam, 5.0, 2_000_000_000, Some(253900));
        let b = session("Other", Source::Steam, 90.0, 3_000_000_000, Some(1));
        let refs = vec![&a, &b];
        let focus = FocusHint {
            class: Some("steam_app_253900".into()),
            ..Default::default()
        };
        let picked = pick_session(&refs, PickPolicy::Focused, Some(&focus)).unwrap();
        assert_eq!(picked.steam_app_id, Some(253900));
    }

    #[test]
    fn parse_steam_class() {
        assert_eq!(parse_steam_app_class("steam_app_253900"), Some(253900));
        assert_eq!(parse_steam_app_class("Steam_App_1"), Some(1));
        assert_eq!(parse_steam_app_class("firefox"), None);
    }

    #[test]
    fn filter_min_rss() {
        let small = session("tiny", Source::Wine, 50.0, 1024, None);
        let big = session("big", Source::Wine, 10.0, 200 * 1024 * 1024, None);
        let snap = SessionSnapshot {
            sessions: vec![small, big],
            orphans: vec![],
            scanned_at: chrono::Utc::now(),
        };
        let report = build_report(&snap, PickPolicy::Hottest, &StatusFilter::default(), None);
        assert!(report.present);
        assert_eq!(report.name.as_deref(), Some("big"));
    }
}
