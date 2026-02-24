use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub struct ProfileMetrics {
    pub total_requests: AtomicU64,
    pub total_tokens: AtomicU64,
    pub success_count: AtomicU64,
    pub failure_count: AtomicU64,
    pub latencies: Mutex<VecDeque<Duration>>,
}

impl ProfileMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            latencies: Mutex::new(VecDeque::with_capacity(100)),
        }
    }

    pub fn record_request(&self, success: bool, latency: Duration, tokens: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_tokens.fetch_add(tokens, Ordering::Relaxed);

        if success {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
        }

        if let Ok(mut lat) = self.latencies.lock() {
            if lat.len() >= 100 {
                lat.pop_front();
            }
            lat.push_back(latency);
        }
    }

    pub fn avg_latency(&self) -> Option<Duration> {
        let lat = self.latencies.lock().ok()?;
        if lat.is_empty() {
            return None;
        }
        let sum: Duration = lat.iter().sum();
        Some(sum / lat.len() as u32)
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 100.0;
        }
        let success = self.success_count.load(Ordering::Relaxed);
        (success as f64 / total as f64) * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct MetricsStore {
    inner: Arc<Mutex<HashMap<String, Arc<ProfileMetrics>>>>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_or_create(&self, profile: &str) -> Arc<ProfileMetrics> {
        let mut map = self.inner.lock().unwrap();
        map.entry(profile.to_string())
            .or_insert_with(|| Arc::new(ProfileMetrics::new()))
            .clone()
    }

    pub fn snapshot(&self) -> HashMap<String, Arc<ProfileMetrics>> {
        self.inner.lock().unwrap().clone()
    }
}
