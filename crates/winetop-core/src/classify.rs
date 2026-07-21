use crate::discover::RawProcess;
use crate::metrics::CpuTracker;
use crate::model::{OrphanInfo, ProcessKind, Session, Source, WineProcess};
use crate::util::{
    appid_from_compatdata_path, env_get, parse_steam_launch_appid, redact_env_key,
    windows_image_from_cmdline,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Whether a raw process looks Wine/Proton related.
pub fn is_wine_related(p: &RawProcess) -> bool {
    if is_steam_client(p) {
        return false;
    }
    let name = p.name.to_ascii_lowercase();
    let cmd = p.cmdline.to_ascii_lowercase();
    let exe = p
        .exe_path
        .as_ref()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    if name == "reaper" && parse_steam_launch_appid(&p.cmdline).is_some() {
        return true;
    }
    if name.contains("wineserver")
        || name.contains("wine-preloader")
        || name.contains("wine64-preloader")
        || name == "wine"
        || name == "wine64"
        || name.starts_with("wine-")
    {
        return true;
    }
    if exe.contains("wine") || exe.contains("proton") {
        return true;
    }
    if cmd.contains("wineserver")
        || cmd.contains("wine-preloader")
        || cmd.contains("/proton")
        || cmd.contains("pressure-vessel")
    {
        return true;
    }
    if env_get(&p.environ, "WINEPREFIX").is_some()
        || env_get(&p.environ, "STEAM_COMPAT_DATA_PATH").is_some()
    {
        // Avoid labeling every random child that inherited env — require wine-ish name/cmd.
        if name.contains("wine")
            || name.ends_with(".exe")
            || cmd.contains(".exe")
            || cmd.contains("wine")
            || cmd.contains("proton")
        {
            return true;
        }
    }
    // Windows PE launched under wine often shows as the exe name via preloader
    if name.ends_with(".exe") && has_wine_ancestor_hint(p) {
        return true;
    }
    false
}

fn is_steam_client(p: &RawProcess) -> bool {
    let name = p.name.to_ascii_lowercase();
    let cmd = p.cmdline.to_ascii_lowercase();
    (name == "steam" || name == "steamwebhelper" || name == "steamservice")
        && !cmd.contains("steamlaunch appid=")
}

fn has_wine_ancestor_hint(p: &RawProcess) -> bool {
    env_get(&p.environ, "WINEPREFIX").is_some()
        || env_get(&p.environ, "WINELOADER").is_some()
        || env_get(&p.environ, "WINESERVER").is_some()
}

pub fn build_sessions(raw: Vec<RawProcess>, tracker: &mut CpuTracker) -> Vec<Session> {
    let related: Vec<&RawProcess> = raw.iter().filter(|p| is_wine_related(p)).collect();
    let live_pids: Vec<u32> = related.iter().map(|p| p.pid).collect();
    tracker.retain_pids(&live_pids);

    let mut wine_procs: Vec<WineProcess> = related
        .iter()
        .map(|p| to_wine_process(p, tracker))
        .collect();

    // Attach reapers that may not have WINEPREFIX yet — keep them.
    // Group by session key.
    let mut groups: HashMap<String, Vec<WineProcess>> = HashMap::new();
    for proc in wine_procs.drain(..) {
        let key = session_key(&proc);
        groups.entry(key).or_default().push(proc);
    }

    let mut sessions: Vec<Session> = groups
        .into_iter()
        .map(|(id, processes)| finalize_session(id, processes))
        .filter(session_is_substantive)
        .collect();

    sessions.sort_by(|a, b| {
        b.cpu_percent
            .partial_cmp(&a.cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    sessions
}

fn to_wine_process(p: &RawProcess, tracker: &mut CpuTracker) -> WineProcess {
    let wine_prefix = env_get(&p.environ, "WINEPREFIX").map(PathBuf::from);
    let steam_compat = env_get(&p.environ, "STEAM_COMPAT_DATA_PATH").map(PathBuf::from);
    let steam_app_id = env_get(&p.environ, "SteamAppId")
        .or_else(|| env_get(&p.environ, "SteamGameId"))
        .and_then(|s| s.parse().ok())
        .or_else(|| parse_steam_launch_appid(&p.cmdline))
        .or_else(|| wine_prefix.as_deref().and_then(appid_from_compatdata_path))
        .or_else(|| {
            steam_compat
                .as_ref()
                .and_then(|p| appid_from_compatdata_path(&p.join("pfx")))
        });
    let lutris_uuid = env_get(&p.environ, "LUTRIS_GAME_UUID");
    let kind = classify_kind(p);
    let is_session_root = kind == ProcessKind::Reaper
        || (parse_steam_launch_appid(&p.cmdline).is_some() && p.name == "reaper");
    let environ_keys: Vec<String> = p
        .environ
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| !redact_env_key(k))
        .collect();
    let windows_image = windows_image_from_cmdline(&p.cmdline).or_else(|| {
        if p.name.to_ascii_lowercase().ends_with(".exe") {
            Some(p.name.clone())
        } else {
            None
        }
    });
    WineProcess {
        pid: p.pid,
        ppid: p.ppid,
        name: p.name.clone(),
        windows_image,
        cmdline: p.cmdline.clone(),
        exe_path: p.exe_path.clone(),
        cwd: p.cwd.clone(),
        kind,
        state: p.state.clone(),
        cpu_percent: tracker.cpu_percent(p.pid, p.cpu_ticks),
        rss_bytes: p.rss_bytes,
        threads: p.threads,
        start_time_secs: p.start_time_secs,
        is_session_root,
        environ_keys,
        wine_prefix,
        steam_app_id,
        steam_compat_data: steam_compat,
        lutris_uuid,
    }
}

fn classify_kind(p: &RawProcess) -> ProcessKind {
    let name = p.name.to_ascii_lowercase();
    if name == "reaper" && parse_steam_launch_appid(&p.cmdline).is_some() {
        return ProcessKind::Reaper;
    }
    if name.contains("wineserver") {
        return ProcessKind::WineServer;
    }
    if name.contains("preloader") || name == "wine" || name == "wine64" {
        return ProcessKind::WineLoader;
    }
    if name.ends_with(".exe") || p.cmdline.to_ascii_lowercase().contains(".exe") {
        return ProcessKind::WindowsExe;
    }
    if p.cmdline.contains("pressure-vessel") || name.contains("proton") {
        return ProcessKind::Helper;
    }
    ProcessKind::Other
}

fn session_key(p: &WineProcess) -> String {
    if let Some(id) = p.steam_app_id {
        return format!("steam:{id}");
    }
    if let Some(ref uuid) = p.lutris_uuid {
        return format!("lutris:{uuid}");
    }
    if let Some(ref prefix) = p.wine_prefix {
        return format!("prefix:{}", prefix.display());
    }
    if let Some(ref compat) = p.steam_compat_data {
        return format!("compat:{}", compat.display());
    }
    // Fall back to process tree root-ish grouping by wineserver pid or self
    format!("pid:{}", p.pid)
}

fn session_is_substantive(s: &Session) -> bool {
    if s.prefix.is_some() || s.steam_app_id.is_some() || s.wineserver_alive {
        return true;
    }
    s.processes.iter().any(|p| {
        matches!(
            p.kind,
            ProcessKind::Reaper
                | ProcessKind::WineServer
                | ProcessKind::WineLoader
                | ProcessKind::WindowsExe
        )
    })
}

fn finalize_session(id: String, mut processes: Vec<WineProcess>) -> Session {
    processes.sort_by_key(|p| (p.kind as u8, p.pid));
    let source = detect_source(&id, &processes);
    let steam_app_id = processes.iter().find_map(|p| p.steam_app_id);
    let prefix = processes
        .iter()
        .find_map(|p| p.wine_prefix.clone())
        .or_else(|| {
            processes
                .iter()
                .find_map(|p| p.steam_compat_data.as_ref().map(|c| c.join("pfx")))
        });
    let runner = detect_runner(&processes);
    let cpu_percent: f32 = processes.iter().map(|p| p.cpu_percent).sum();
    let rss_bytes: u64 = processes.iter().map(|p| p.rss_bytes).sum();
    let wineserver_alive = processes.iter().any(|p| p.kind == ProcessKind::WineServer);
    let name = default_session_name(source, steam_app_id, &prefix, &processes);
    let mut notes = Vec::new();
    if source == Source::Bottles {
        notes.push("Bottles session detected from path/env".into());
    }
    Session {
        id,
        source,
        name,
        prefix,
        steam_app_id,
        runner,
        processes,
        cpu_percent,
        rss_bytes,
        wineserver_alive,
        opaque: false,
        notes,
    }
}

fn detect_source(id: &str, processes: &[WineProcess]) -> Source {
    if id.starts_with("steam:") || processes.iter().any(|p| p.steam_app_id.is_some()) {
        return Source::Steam;
    }
    if processes.iter().any(|p| p.lutris_uuid.is_some()) {
        return Source::Lutris;
    }
    for p in processes {
        let hay = format!(
            "{} {} {}",
            p.cmdline,
            p.exe_path
                .as_ref()
                .map(|x| x.display().to_string())
                .unwrap_or_default(),
            p.wine_prefix
                .as_ref()
                .map(|x| x.display().to_string())
                .unwrap_or_default()
        );
        let lower = hay.to_ascii_lowercase();
        if lower.contains("/lutris/") || lower.contains("lutris/runners") {
            return Source::Lutris;
        }
        if lower.contains("heroic") || lower.contains("legendary") || lower.contains("gogdl") {
            return Source::Heroic;
        }
        if lower.contains("bottles") || lower.contains("usebottles") {
            return Source::Bottles;
        }
        if lower.contains("proton") || lower.contains("compatdata") {
            return Source::Steam;
        }
    }
    Source::Wine
}

fn detect_runner(processes: &[WineProcess]) -> Option<String> {
    for p in processes {
        if let Some(ref exe) = p.exe_path {
            let s = exe.display().to_string();
            if s.contains("Proton") {
                // …/Proton - Experimental/files/bin/wine64
                if let Some(idx) = s.find("Proton") {
                    let rest = &s[idx..];
                    let name = rest.split('/').next().unwrap_or("Proton");
                    return Some(name.to_string());
                }
            }
            if s.contains("lutris/runners/wine/") {
                if let Some(idx) = s.find("runners/wine/") {
                    let rest = &s[idx + "runners/wine/".len()..];
                    let name = rest.split('/').next().unwrap_or("lutris-wine");
                    return Some(name.to_string());
                }
            }
            if s.contains("wine") {
                return Some(
                    exe.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "wine".into()),
                );
            }
        }
    }
    None
}

fn default_session_name(
    source: Source,
    appid: Option<u32>,
    prefix: &Option<PathBuf>,
    processes: &[WineProcess],
) -> String {
    if let Some(id) = appid {
        return format!("Steam App {id}");
    }
    if let Some(img) = processes.iter().find_map(|p| {
        p.windows_image.as_ref().and_then(|w| {
            let lower = w.to_ascii_lowercase();
            if lower == "steam.exe"
                || lower.contains("gameoverlay")
                || lower == "explorer.exe"
                || lower == "services.exe"
            {
                None
            } else {
                Some(w.clone())
            }
        })
    }) {
        return img;
    }
    if let Some(p) = prefix {
        if let Some(name) = p.file_name() {
            return name.to_string_lossy().into_owned();
        }
    }
    source.as_str().to_string()
}

pub fn find_orphans(sessions: &[Session]) -> Vec<OrphanInfo> {
    let mut orphans = Vec::new();
    for s in sessions {
        let has_clients = s.processes.iter().any(|p| {
            matches!(
                p.kind,
                ProcessKind::WindowsExe | ProcessKind::WineLoader | ProcessKind::Helper
            )
        });
        if s.wineserver_alive && !has_clients {
            if let Some(ws) = s
                .processes
                .iter()
                .find(|p| p.kind == ProcessKind::WineServer)
            {
                orphans.push(OrphanInfo {
                    kind: "lonely_wineserver".into(),
                    pid: Some(ws.pid),
                    prefix: s.prefix.clone(),
                    detail: format!("wineserver {} with no clients in {}", ws.pid, s.name),
                });
            }
        }
        for p in &s.processes {
            if p.state == "Z" || p.state.to_ascii_lowercase().starts_with('z') {
                orphans.push(OrphanInfo {
                    kind: "zombie".into(),
                    pid: Some(p.pid),
                    prefix: s.prefix.clone(),
                    detail: format!("zombie {} ({})", p.pid, p.name),
                });
            }
        }
        // Leftover wine procs without reaper for Steam sessions
        if s.source == Source::Steam && s.reaper_pid().is_none() && !s.processes.is_empty() {
            if let Some(appid) = s.steam_app_id {
                orphans.push(OrphanInfo {
                    kind: "missing_reaper".into(),
                    pid: None,
                    prefix: s.prefix.clone(),
                    detail: format!("Steam session {appid} has Wine procs but no reaper"),
                });
            }
        }
    }
    orphans
}

/// Build a simple parent→children tree of wine-related pids for display.
pub fn process_tree_lines(sessions: &[Session]) -> Vec<String> {
    let mut by_pid: HashMap<u32, &WineProcess> = HashMap::new();
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut all: HashSet<u32> = HashSet::new();
    for s in sessions {
        for p in &s.processes {
            by_pid.insert(p.pid, p);
            all.insert(p.pid);
            children.entry(p.ppid).or_default().push(p.pid);
        }
    }
    for v in children.values_mut() {
        v.sort_unstable();
    }
    let roots: Vec<u32> = all
        .iter()
        .copied()
        .filter(|pid| !all.contains(&by_pid[pid].ppid))
        .collect();
    let mut lines = Vec::new();
    for root in roots {
        walk_tree(root, "", true, &by_pid, &children, &mut lines);
    }
    lines
}

fn walk_tree(
    pid: u32,
    prefix: &str,
    is_last: bool,
    by_pid: &HashMap<u32, &WineProcess>,
    children: &HashMap<u32, Vec<u32>>,
    lines: &mut Vec<String>,
) {
    let Some(p) = by_pid.get(&pid) else {
        return;
    };
    let branch = if prefix.is_empty() {
        ""
    } else if is_last {
        "└─ "
    } else {
        "├─ "
    };
    let label = p.windows_image.clone().unwrap_or_else(|| p.name.clone());
    lines.push(format!(
        "{prefix}{branch}{label:<28} {pid:<7} {:>5.1}%  {}",
        p.cpu_percent,
        crate::util::format_bytes(p.rss_bytes)
    ));
    let kids = children.get(&pid).cloned().unwrap_or_default();
    let next_prefix = if prefix.is_empty() {
        String::new()
    } else if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}│  ")
    };
    // For roots use empty→indent for children
    let child_prefix = if prefix.is_empty() && branch.is_empty() {
        String::new()
    } else {
        next_prefix
    };
    let child_prefix = if prefix.is_empty() {
        "   ".to_string()
    } else {
        child_prefix
    };
    let n = kids.len();
    for (i, child) in kids.into_iter().enumerate() {
        walk_tree(child, &child_prefix, i + 1 == n, by_pid, children, lines);
    }
}

pub fn path_looks_like_prefix(path: &Path) -> bool {
    path.join("drive_c").is_dir()
        && (path.join("system.reg").is_file() || path.join("user.reg").is_file())
}
