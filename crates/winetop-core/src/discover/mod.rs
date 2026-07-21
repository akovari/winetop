//! Platform process discovery.

mod raw;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "freebsd")]
mod freebsd;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
mod stub;

pub use raw::{scan_processes, RawProcess};
