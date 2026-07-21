//! macOS / CrossOver best-effort discovery via `ps`.
//!
//! SIP limits environ access; we classify primarily from cmdline and known
//! CrossOver bottle path patterns.

use super::raw::RawProcess;
use crate::{Error, Result};
use std::process::Command;

pub fn scan() -> Result<Vec<RawProcess>> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,rss=,state=,command="])
        .output()
        .map_err(|e| Error::Other(format!("ps failed: {e}")))?;
    if !output.status.success() {
        return Err(Error::Other("ps returned non-zero".into()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in text.lines() {
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
        let cmdline = parts.collect::<Vec<_>>().join(" ");
        let name = cmdline
            .split_whitespace()
            .next()
            .unwrap_or("?")
            .rsplit('/')
            .next()
            .unwrap_or("?")
            .to_string();
        let mut environ = Vec::new();
        // CrossOver bottles often appear in paths like .../Bottles/<name>/...
        if let Some(idx) = cmdline.find("/Bottles/") {
            let rest = &cmdline[idx + "/Bottles/".len()..];
            let bottle = rest.split('/').next().unwrap_or("bottle");
            if let Some(home) = dirs::home_dir() {
                let prefix = home
                    .join("Library/Application Support/CrossOver/Bottles")
                    .join(bottle);
                environ.push(("WINEPREFIX".into(), prefix.display().to_string()));
            }
        }
        for token in cmdline.split_whitespace() {
            if let Some(v) = token.strip_prefix("WINEPREFIX=") {
                environ.push(("WINEPREFIX".into(), v.into()));
            }
        }
        out.push(RawProcess {
            pid,
            ppid,
            name,
            cmdline,
            exe_path: None,
            cwd: None,
            state,
            rss_bytes: rss_kb * 1024,
            threads: 1,
            start_time_secs: 0,
            cpu_ticks: 0,
            environ,
        });
    }
    Ok(out)
}
