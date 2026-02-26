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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metrics_are_zero() {
        let m = ProfileMetrics::new();
        assert_eq!(m.total_requests.load(Ordering::Relaxed), 0);
        assert_eq!(m.success_count.load(Ordering::Relaxed), 0);
        assert_eq!(m.failure_count.load(Ordering::Relaxed), 0);
        assert_eq!(m.total_tokens.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_success() {
        let m = ProfileMetrics::new();
        m.record_request(true, Duration::from_millis(100), 50);

        assert_eq!(m.total_requests.load(Ordering::Relaxed), 1);
        assert_eq!(m.success_count.load(Ordering::Relaxed), 1);
        assert_eq!(m.failure_count.load(Ordering::Relaxed), 0);
        assert_eq!(m.total_tokens.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_record_failure() {
        let m = ProfileMetrics::new();
        m.record_request(false, Duration::from_millis(200), 0);

        assert_eq!(m.total_requests.load(Ordering::Relaxed), 1);
        assert_eq!(m.success_count.load(Ordering::Relaxed), 0);
        assert_eq!(m.failure_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_avg_latency_empty() {
        let m = ProfileMetrics::new();
        assert!(m.avg_latency().is_none());
    }

    #[test]
    fn test_avg_latency_single() {
        let m = ProfileMetrics::new();
        m.record_request(true, Duration::from_millis(100), 0);
        assert_eq!(m.avg_latency(), Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_avg_latency_multiple() {
        let m = ProfileMetrics::new();
        m.record_request(true, Duration::from_millis(100), 0);
        m.record_request(true, Duration::from_millis(200), 0);
        m.record_request(true, Duration::from_millis(300), 0);
        assert_eq!(m.avg_latency(), Some(Duration::from_millis(200)));
    }

    #[test]
    fn test_latency_buffer_cap_at_100() {
        let m = ProfileMetrics::new();
        for i in 0..110 {
            m.record_request(true, Duration::from_millis(i), 0);
        }
        let lat = m.latencies.lock().unwrap();
        assert_eq!(lat.len(), 100);
        // Should have dropped first 10, so earliest should be 10ms
        assert_eq!(*lat.front().unwrap(), Duration::from_millis(10));
    }

    #[test]
    fn test_success_rate_no_requests() {
        let m = ProfileMetrics::new();
        assert_eq!(m.success_rate(), 100.0);
    }

    #[test]
    fn test_success_rate_mixed() {
        let m = ProfileMetrics::new();
        m.record_request(true, Duration::from_millis(10), 0);
        m.record_request(true, Duration::from_millis(10), 0);
        m.record_request(false, Duration::from_millis(10), 0);
        // 2/3 = 66.67%
        let rate = m.success_rate();
        assert!((rate - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_metrics_store_get_or_create() {
        let store = MetricsStore::new();
        let m1 = store.get_or_create("grok");
        m1.record_request(true, Duration::from_millis(50), 10);

        let m2 = store.get_or_create("grok");
        assert_eq!(m2.total_requests.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_metrics_store_snapshot() {
        let store = MetricsStore::new();
        store.get_or_create("a");
        store.get_or_create("b");

        let snap = store.snapshot();
        assert_eq!(snap.len(), 2);
        assert!(snap.contains_key("a"));
        assert!(snap.contains_key("b"));
    }
}
