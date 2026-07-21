use crate::Result;
use std::path::PathBuf;

/// Platform-agnostic process record before Wine classification.
#[derive(Debug, Clone)]
pub struct RawProcess {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub cmdline: String,
    pub exe_path: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub state: String,
    pub rss_bytes: u64,
    pub threads: u32,
    pub start_time_secs: u64,
    /// utime + stime in clock ticks (Linux) or equivalent.
    pub cpu_ticks: u64,
    pub environ: Vec<(String, String)>,
}

pub fn scan_processes() -> Result<Vec<RawProcess>> {
    #[cfg(target_os = "linux")]
    {
        crate::discover::linux::scan()
    }
    #[cfg(target_os = "freebsd")]
    {
        crate::discover::freebsd::scan()
    }
    #[cfg(target_os = "macos")]
    {
        crate::discover::macos::scan()
    }
    #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
    {
        crate::discover::stub::scan()
    }
}
