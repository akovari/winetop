use super::raw::RawProcess;
use crate::Result;
use std::fs;
use std::path::PathBuf;

pub fn scan() -> Result<Vec<RawProcess>> {
    let mut out = Vec::new();
    let proc = PathBuf::from("/proc");
    let entries = fs::read_dir(&proc)?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let pid: u32 = match name.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        if let Some(p) = read_process(pid) {
            out.push(p);
        }
    }
    Ok(out)
}

fn read_process(pid: u32) -> Option<RawProcess> {
    let base = PathBuf::from(format!("/proc/{pid}"));
    let stat = fs::read_to_string(base.join("stat")).ok()?;
    let (ppid, state, utime, stime, starttime, threads) = parse_stat(&stat)?;
    let status = fs::read_to_string(base.join("status")).unwrap_or_default();
    let rss_bytes = parse_status_vmrss(&status).unwrap_or(0) * 1024;
    let cmdline_raw = fs::read(base.join("cmdline")).unwrap_or_default();
    let cmdline = cmdline_raw
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let comm = fs::read_to_string(base.join("comm"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let name = if comm.is_empty() {
        cmdline
            .split_whitespace()
            .next()
            .unwrap_or("?")
            .rsplit('/')
            .next()
            .unwrap_or("?")
            .to_string()
    } else {
        comm
    };
    let exe_path = fs::read_link(base.join("exe")).ok();
    let cwd = fs::read_link(base.join("cwd")).ok();
    let environ = read_environ(&base.join("environ"));
    let clock_ticks = 100u64; // USER_HZ typical
    let start_time_secs = starttime / clock_ticks;
    Some(RawProcess {
        pid,
        ppid,
        name,
        cmdline,
        exe_path,
        cwd,
        state: state.to_string(),
        rss_bytes,
        threads,
        start_time_secs,
        cpu_ticks: utime.saturating_add(stime),
        environ,
    })
}

fn parse_stat(stat: &str) -> Option<(u32, char, u64, u64, u64, u32)> {
    // Format: pid (comm) state ppid ...
    let comm_end = stat.rfind(')')?;
    let after = stat[comm_end + 2..].trim_start();
    let mut parts = after.split_whitespace();
    let state = parts.next()?.chars().next()?;
    let ppid: u32 = parts.next()?.parse().ok()?;
    // fields: 5=pgrp ... we need utime(14), stime(15), threads(20), starttime(22)
    // After state/ppid we are at field 5 in man proc.
    let rest: Vec<&str> = parts.collect();
    // rest[0] is pgrp (field 5), so:
    // utime = rest[11] (field 14), stime = rest[12], threads = rest[17], starttime = rest[19]
    if rest.len() < 20 {
        return None;
    }
    let utime: u64 = rest[11].parse().ok()?;
    let stime: u64 = rest[12].parse().ok()?;
    let threads: u32 = rest[17].parse().ok()?;
    let starttime: u64 = rest[19].parse().ok()?;
    Some((ppid, state, utime, stime, starttime, threads))
}

fn parse_status_vmrss(status: &str) -> Option<u64> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb);
        }
    }
    None
}

fn read_environ(path: &std::path::Path) -> Vec<(String, String)> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    bytes
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .filter_map(|s| {
            let s = String::from_utf8_lossy(s);
            let (k, v) = s.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stat_self() {
        let stat = fs::read_to_string(format!("/proc/{}/stat", std::process::id())).unwrap();
        let parsed = parse_stat(&stat);
        assert!(parsed.is_some());
    }
}
