use crate::model::{Session, SessionSnapshot};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillMethod {
    /// SIGTERM a single PID.
    ProcessTerm,
    /// SIGKILL a single PID.
    ProcessKill,
    /// SIGTERM Steam reaper for the session (preferred for Steam games).
    SessionReaper,
    /// `WINEPREFIX=… wineserver -k`
    WineServerK,
    /// SIGKILL every process in the session.
    SessionKill,
}

#[derive(Debug, Clone)]
pub struct KillRequest {
    pub method: KillMethod,
    pub pid: Option<u32>,
    pub session_id: Option<String>,
    pub steam_app_id: Option<u32>,
    pub prefix: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillResult {
    pub ok: bool,
    pub message: String,
    pub affected_pids: Vec<u32>,
}

pub fn execute(req: &KillRequest, snap: &SessionSnapshot) -> Result<KillResult> {
    match req.method {
        KillMethod::ProcessTerm => {
            let pid = req.pid.ok_or_else(|| Error::Other("pid required".into()))?;
            signal_pid(pid, SignalKind::Term)?;
            Ok(KillResult {
                ok: true,
                message: format!("sent SIGTERM to {pid}"),
                affected_pids: vec![pid],
            })
        }
        KillMethod::ProcessKill => {
            let pid = req.pid.ok_or_else(|| Error::Other("pid required".into()))?;
            signal_pid(pid, SignalKind::Kill)?;
            Ok(KillResult {
                ok: true,
                message: format!("sent SIGKILL to {pid}"),
                affected_pids: vec![pid],
            })
        }
        KillMethod::SessionReaper => {
            let session = resolve_session(req, snap)?;
            stop_session_reaper(session)
        }
        KillMethod::WineServerK => {
            let session = resolve_session(req, snap)?;
            let prefix = session
                .prefix
                .clone()
                .or_else(|| req.prefix.clone())
                .ok_or_else(|| Error::Other("no prefix for wineserver -k".into()))?;
            wineserver_k(&prefix)
        }
        KillMethod::SessionKill => {
            let session = resolve_session(req, snap)?;
            hard_kill_session(session)
        }
    }
}

fn resolve_session<'a>(req: &KillRequest, snap: &'a SessionSnapshot) -> Result<&'a Session> {
    if let Some(ref id) = req.session_id {
        return snap
            .find_session(id)
            .ok_or_else(|| Error::SessionNotFound(id.clone()));
    }
    if let Some(appid) = req.steam_app_id {
        return snap
            .find_by_appid(appid)
            .ok_or_else(|| Error::SessionNotFound(format!("steam:{appid}")));
    }
    if let Some(ref prefix) = req.prefix {
        return snap
            .find_by_prefix(prefix)
            .ok_or_else(|| Error::SessionNotFound(prefix.display().to_string()));
    }
    Err(Error::Other(
        "session_id, steam_app_id, or prefix required".into(),
    ))
}

/// Preferred Steam stop: SIGTERM reaper, then wineserver -k if leftovers.
pub fn stop_session_graceful(session: &Session) -> Result<KillResult> {
    let mut affected = Vec::new();
    let mut messages = Vec::new();

    if let Some(reaper) = session.reaper_pid() {
        match signal_pid(reaper, SignalKind::Term) {
            Ok(()) => {
                affected.push(reaper);
                messages.push(format!("SIGTERM reaper {reaper}"));
                thread::sleep(Duration::from_millis(800));
            }
            Err(e) => {
                warn!("reaper term failed: {e}");
                messages.push(format!("reaper term failed: {e}"));
            }
        }
    }

    if let Some(ref prefix) = session.prefix {
        match wineserver_k(prefix) {
            Ok(r) => {
                affected.extend(r.affected_pids);
                messages.push(r.message);
            }
            Err(e) => messages.push(format!("wineserver -k: {e}")),
        }
    }

    Ok(KillResult {
        ok: true,
        message: messages.join("; "),
        affected_pids: affected,
    })
}

fn stop_session_reaper(session: &Session) -> Result<KillResult> {
    stop_session_graceful(session)
}

fn hard_kill_session(session: &Session) -> Result<KillResult> {
    let mut affected = Vec::new();
    for p in &session.processes {
        // Never kill a process named exactly steam (client)
        if p.name.eq_ignore_ascii_case("steam") && p.kind != crate::model::ProcessKind::Reaper {
            continue;
        }
        if signal_pid(p.pid, SignalKind::Kill).is_ok() {
            affected.push(p.pid);
        }
    }
    Ok(KillResult {
        ok: true,
        message: format!("SIGKILL {} processes in {}", affected.len(), session.name),
        affected_pids: affected,
    })
}

pub fn wineserver_k(prefix: &Path) -> Result<KillResult> {
    info!(prefix = %prefix.display(), "running wineserver -k");
    let wineserver = find_wineserver_for_prefix(prefix);
    let mut cmd = Command::new(&wineserver);
    cmd.arg("-k");
    cmd.env("WINEPREFIX", prefix);
    let output = cmd.output().map_err(|e| Error::WineServerKill {
        prefix: prefix.display().to_string(),
        message: e.to_string(),
    })?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let msg = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("wineserver -k on {}", prefix.display())
    };
    Ok(KillResult {
        ok: output.status.success() || output.status.code() == Some(0),
        message: msg,
        affected_pids: vec![],
    })
}

fn find_wineserver_for_prefix(_prefix: &Path) -> String {
    // Prefer PATH wineserver; runners may set WINESERVER in live procs later.
    which("wineserver").unwrap_or_else(|| "wineserver".into())
}

fn which(bin: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate.display().to_string());
        }
    }
    None
}

#[derive(Clone, Copy)]
enum SignalKind {
    Term,
    Kill,
}

fn signal_pid(pid: u32, kind: SignalKind) -> Result<()> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        let sig = match kind {
            SignalKind::Term => Signal::SIGTERM,
            SignalKind::Kill => Signal::SIGKILL,
        };
        kill(Pid::from_raw(pid as i32), sig).map_err(|e| Error::KillFailed {
            pid,
            message: e.to_string(),
        })?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, kind);
        Err(Error::UnsupportedPlatform)
    }
}

/// Stop all sessions via per-prefix wineserver -k (nuclear but safer than killall).
pub fn kill_all_prefixes(snap: &SessionSnapshot) -> Result<KillResult> {
    let mut messages = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for s in &snap.sessions {
        if let Some(ref prefix) = s.prefix {
            let key = prefix.display().to_string();
            if !seen.insert(key) {
                continue;
            }
            match wineserver_k(prefix) {
                Ok(r) => messages.push(r.message),
                Err(e) => messages.push(e.to_string()),
            }
        } else if let Some(reaper) = s.reaper_pid() {
            let _ = signal_pid(reaper, SignalKind::Term);
            messages.push(format!("SIGTERM reaper {reaper}"));
        }
    }
    Ok(KillResult {
        ok: true,
        message: messages.join("; "),
        affected_pids: vec![],
    })
}
