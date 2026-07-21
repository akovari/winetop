//! FreeBSD process discovery via `ps` / best-effort sysctl-friendly CLI.
//!
//! Full `libprocstat` binding is future work; this port uses `ps` output so
//! winetop still builds and runs usefully on FreeBSD.

use super::raw::RawProcess;
use crate::{Error, Result};
use std::process::Command;

pub fn scan() -> Result<Vec<RawProcess>> {
    let output = Command::new("ps")
        .args(["-axwwo", "pid,ppid,rss,state,time,command"])
        .output()
        .map_err(|e| Error::Other(format!("ps failed: {e}")))?;
    if !output.status.success() {
        return Err(Error::Other("ps returned non-zero".into()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(pid) = parts.next().and_then(|s| s.parse().ok()) else {
            continue;
        };
        let Some(ppid) = parts.next().and_then(|s| s.parse().ok()) else {
            continue;
        };
        let rss_kb: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let state = parts.next().unwrap_or("?").to_string();
        let _time = parts.next();
        let cmdline = parts.collect::<Vec<_>>().join(" ");
        let name = cmdline
            .split_whitespace()
            .next()
            .unwrap_or("?")
            .rsplit('/')
            .next()
            .unwrap_or("?")
            .to_string();
        // Environ is not available without procfs/libprocstat — classify by cmdline/paths.
        out.push(RawProcess {
            pid,
            ppid,
            name,
            cmdline: cmdline.clone(),
            exe_path: None,
            cwd: None,
            state,
            rss_bytes: rss_kb * 1024,
            threads: 1,
            start_time_secs: 0,
            cpu_ticks: 0,
            environ: infer_environ_from_cmdline(&cmdline),
        });
    }
    Ok(out)
}

fn infer_environ_from_cmdline(cmdline: &str) -> Vec<(String, String)> {
    let mut env = Vec::new();
    for token in cmdline.split_whitespace() {
        if let Some(v) = token.strip_prefix("WINEPREFIX=") {
            env.push(("WINEPREFIX".into(), v.into()));
        }
        if let Some(v) = token.strip_prefix("STEAM_COMPAT_DATA_PATH=") {
            env.push(("STEAM_COMPAT_DATA_PATH".into(), v.into()));
        }
    }
    env
}
