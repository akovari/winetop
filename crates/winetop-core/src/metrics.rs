use std::collections::HashMap;
use std::time::Instant;

/// Tracks previous CPU jiffies / times to compute percent deltas.
#[derive(Debug, Default)]
pub struct CpuTracker {
    prev: HashMap<u32, (u64, Instant)>,
    /// Approximate ticks per second; Linux USER_HZ is typically 100.
    ticks_per_sec: f64,
}

impl CpuTracker {
    pub fn new() -> Self {
        Self {
            prev: HashMap::new(),
            ticks_per_sec: 100.0,
        }
    }

    /// Compute CPU% from cumulative ticks (utime+stime) since boot.
    pub fn cpu_percent(&mut self, pid: u32, total_ticks: u64) -> f32 {
        let now = Instant::now();
        let percent = if let Some((prev_ticks, prev_at)) = self.prev.get(&pid) {
            let dt = now.duration_since(*prev_at).as_secs_f64();
            if dt > 0.0 && total_ticks >= *prev_ticks {
                let delta = (total_ticks - prev_ticks) as f64;
                ((delta / self.ticks_per_sec) / dt * 100.0) as f32
            } else {
                0.0
            }
        } else {
            0.0
        };
        self.prev.insert(pid, (total_ticks, now));
        percent.max(0.0)
    }

    pub fn retain_pids(&mut self, live: &[u32]) {
        self.prev.retain(|pid, _| live.contains(pid));
    }
}
