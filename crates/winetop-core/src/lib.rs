//! Core discovery, session model, and kill engine for winetop.

pub mod classify;
pub mod discover;
pub mod enrich;
pub mod error;
pub mod kill;
pub mod metrics;
pub mod model;
pub mod util;

pub use error::{Error, Result};
pub use kill::{KillMethod, KillRequest, KillResult};
pub use model::{ProcessKind, Session, SessionSnapshot, Source, WineProcess};

use enrich::Enricher;
use metrics::CpuTracker;

/// Scan the host for Wine/Proton-related sessions.
pub fn scan() -> Result<SessionSnapshot> {
    let mut tracker = CpuTracker::new();
    scan_with(&mut tracker)
}

/// Scan using a persistent CPU tracker for accurate deltas between refreshes.
pub fn scan_with(tracker: &mut CpuTracker) -> Result<SessionSnapshot> {
    let raw = discover::scan_processes()?;
    let mut sessions = classify::build_sessions(raw, tracker);
    let enricher = Enricher::load();
    enricher.apply(&mut sessions);
    let orphans = classify::find_orphans(&sessions);
    Ok(SessionSnapshot {
        sessions,
        orphans,
        scanned_at: chrono::Utc::now(),
    })
}
