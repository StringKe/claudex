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
            tracing::warn!(
                failures = self.failure_count,
                "circuit breaker opened"
            );
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
