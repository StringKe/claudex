use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u32,
    pub last_failure: Option<Instant>,
    pub threshold: u32,
    pub recovery_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
            threshold,
            recovery_timeout,
        }
    }

    pub fn can_attempt(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last) = self.last_failure {
                    if last.elapsed() >= self.recovery_timeout {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.threshold {
            self.state = CircuitState::Open;
            tracing::warn!(failures = self.failure_count, "circuit breaker opened");
        }
    }

    pub fn is_open(&self) -> bool {
        self.state == CircuitState::Open
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(30))
    }
}

pub type CircuitBreakerMap = Arc<RwLock<HashMap<String, CircuitBreaker>>>;

pub fn new_circuit_breaker_map() -> CircuitBreakerMap {
    Arc::new(RwLock::new(HashMap::new()))
}

#[allow(dead_code)]
pub async fn get_or_create(map: &CircuitBreakerMap, profile: &str) -> CircuitBreaker {
    let read = map.read().await;
    if let Some(cb) = read.get(profile) {
        return CircuitBreaker {
            state: cb.state.clone(),
            failure_count: cb.failure_count,
            last_failure: cb.last_failure,
            threshold: cb.threshold,
            recovery_timeout: cb.recovery_timeout,
        };
    }
    drop(read);

    let mut write = map.write().await;
    write
        .entry(profile.to_string())
        .or_insert_with(CircuitBreaker::default);
    let cb = write.get(profile).unwrap();
    CircuitBreaker {
        state: cb.state.clone(),
        failure_count: cb.failure_count,
        last_failure: cb.last_failure,
        threshold: cb.threshold,
        recovery_timeout: cb.recovery_timeout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.state, CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
        assert_eq!(cb.threshold, 3);
        assert_eq!(cb.recovery_timeout, Duration::from_secs(30));
        assert!(cb.last_failure.is_none());
    }

    #[test]
    fn test_closed_can_attempt() {
        let mut cb = CircuitBreaker::default();
        assert!(cb.can_attempt());
    }

    #[test]
    fn test_single_failure_stays_closed() {
        let mut cb = CircuitBreaker::default();
        cb.record_failure();
        assert_eq!(cb.failure_count, 1);
        assert_eq!(cb.state, CircuitState::Closed);
        assert!(cb.can_attempt());
    }

    #[test]
    fn test_threshold_opens_circuit() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Closed);
        cb.record_failure(); // hits threshold
        assert_eq!(cb.state, CircuitState::Open);
        assert!(cb.is_open());
    }

    #[test]
    fn test_open_rejects_attempts() {
        let mut cb = CircuitBreaker::new(2, Duration::from_secs(60));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitState::Open);
        assert!(!cb.can_attempt()); // should reject
    }

    #[test]
    fn test_success_resets_state() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count, 2);

        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn test_halfopen_after_timeout() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(0)); // instant recovery
        cb.record_failure(); // opens circuit
        assert_eq!(cb.state, CircuitState::Open);

        // With 0ms timeout, should immediately transition to HalfOpen
        assert!(cb.can_attempt());
        assert_eq!(cb.state, CircuitState::HalfOpen);
    }

    #[test]
    fn test_halfopen_allows_attempt() {
        let mut cb = CircuitBreaker::default();
        cb.state = CircuitState::HalfOpen;
        assert!(cb.can_attempt());
    }

    #[test]
    fn test_halfopen_success_closes() {
        let mut cb = CircuitBreaker::default();
        cb.state = CircuitState::HalfOpen;
        cb.record_success();
        assert_eq!(cb.state, CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn test_halfopen_failure_reopens() {
        let mut cb = CircuitBreaker::new(1, Duration::from_secs(30));
        cb.state = CircuitState::HalfOpen;
        cb.failure_count = 0;
        cb.record_failure(); // threshold=1, so this reopens
        assert_eq!(cb.state, CircuitState::Open);
    }

    #[test]
    fn test_is_open() {
        let mut cb = CircuitBreaker::default();
        assert!(!cb.is_open());
        cb.state = CircuitState::Open;
        assert!(cb.is_open());
        cb.state = CircuitState::HalfOpen;
        assert!(!cb.is_open());
    }
}
