use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// In-memory sliding window rate limiter (per IP).
/// Thread-safe via parking_lot::Mutex for minimal contention.
pub struct RateLimiter {
    /// Map: IP → Vec of request timestamps
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    /// Window duration (e.g., 60 seconds)
    window: Duration,
    /// Max requests per window
    max_requests: u32,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        RateLimiter {
            requests: Mutex::new(HashMap::new()),
            window: Duration::from_secs(window_secs),
            max_requests,
        }
    }

    /// Returns true if the request is allowed, false if rate limited.
    /// Also prunes old entries to prevent unbounded memory growth.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.requests.lock();
        let entries = map.entry(key.to_string()).or_insert_with(Vec::new);

        // Prune entries older than window
        entries.retain(|t| now.duration_since(*t) < self.window);

        if entries.len() >= self.max_requests as usize {
            return false;
        }

        entries.push(now);
        true
    }

    /// Periodic cleanup of stale IPs (call from background task)
    pub fn cleanup(&self) {
        let now = Instant::now();
        let mut map = self.requests.lock();
        map.retain(|_, entries| {
            entries.retain(|t| now.duration_since(*t) < self.window);
            !entries.is_empty()
        });
    }
}
