use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Origin launcher / environment for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Steam,
    Lutris,
    Heroic,
    Bottles,
    Wine,
    Unknown,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steam => "Steam",
            Self::Lutris => "Lutris",
            Self::Heroic => "Heroic",
            Self::Bottles => "Bottles",
            Self::Wine => "Wine",
            Self::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    Reaper,
    WineServer,
    WineLoader,
    WindowsExe,
    Helper,
    Other,
}

impl ProcessKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reaper => "reaper",
            Self::WineServer => "wineserver",
            Self::WineLoader => "loader",
            Self::WindowsExe => "exe",
            Self::Helper => "helper",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineProcess {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub windows_image: Option<String>,
    pub cmdline: String,
    pub exe_path: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub kind: ProcessKind,
    pub state: String,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub threads: u32,
    pub start_time_secs: u64,
    pub is_session_root: bool,
    pub environ_keys: Vec<String>,
    #[serde(skip)]
    pub wine_prefix: Option<PathBuf>,
    #[serde(skip)]
    pub steam_app_id: Option<u32>,
    #[serde(skip)]
    pub steam_compat_data: Option<PathBuf>,
    #[serde(skip)]
    pub lutris_uuid: Option<String>,
}

impl WineProcess {
    /// Display name: Windows image if known, else process name.
    pub fn display_name(&self) -> &str {
        self.windows_image.as_deref().unwrap_or(&self.name)
    }

    /// Short cmdline / path+args to distinguish duplicate images.
    pub fn detail(&self) -> String {
        crate::util::cmdline_detail(&self.cmdline)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub source: Source,
    pub name: String,
    pub prefix: Option<PathBuf>,
    pub steam_app_id: Option<u32>,
    pub runner: Option<String>,
    pub processes: Vec<WineProcess>,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub wineserver_alive: bool,
    pub opaque: bool,
    pub notes: Vec<String>,
}

impl Session {
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn reaper_pid(&self) -> Option<u32> {
        self.processes
            .iter()
            .find(|p| p.kind == ProcessKind::Reaper || p.is_session_root)
            .map(|p| p.pid)
    }

    pub fn short_prefix(&self) -> String {
        match &self.prefix {
            Some(p) => util_short_path(p),
            None => "-".into(),
        }
    }
}

fn util_short_path(p: &std::path::Path) -> String {
    let s = p.display().to_string();
    if let Some(home) = dirs::home_dir() {
        let home_s = home.display().to_string();
        if let Some(rest) = s.strip_prefix(&home_s) {
            return format!("~{rest}");
        }
    }
    if let Some(idx) = s.find("compatdata/") {
        return s[idx..].to_string();
    }
    if s.len() > 48 {
        let start = s.len().saturating_sub(45);
        return format!("…{}", &s[start..]);
    }
    s
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanInfo {
    pub kind: String,
    pub pid: Option<u32>,
    pub prefix: Option<PathBuf>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub sessions: Vec<Session>,
    pub orphans: Vec<OrphanInfo>,
    pub scanned_at: DateTime<Utc>,
}

impl SessionSnapshot {
    pub fn totals(&self) -> (usize, f32, u64) {
        let procs: usize = self.sessions.iter().map(|s| s.process_count()).sum();
        let cpu: f32 = self.sessions.iter().map(|s| s.cpu_percent).sum();
        let rss: u64 = self.sessions.iter().map(|s| s.rss_bytes).sum();
        (procs, cpu, rss)
    }

    pub fn find_session(&self, id: &str) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn find_by_appid(&self, appid: u32) -> Option<&Session> {
        self.sessions.iter().find(|s| s.steam_app_id == Some(appid))
    }

    pub fn find_by_prefix(&self, prefix: &std::path::Path) -> Option<&Session> {
        self.sessions.iter().find(|s| {
            s.prefix
                .as_ref()
                .is_some_and(|p| p == prefix || p.ends_with(prefix) || prefix.ends_with(p))
        })
    }
}
