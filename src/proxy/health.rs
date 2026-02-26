use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::proxy::ProxyState;

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub latency_ms: Option<u128>,
    pub last_check: Option<std::time::Instant>,
    pub error: Option<String>,
}

pub type HealthMap = HashMap<String, HealthStatus>;

pub fn spawn_health_checker(state: Arc<ProxyState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            check_all_profiles(&state).await;
        }
    });
}

async fn check_all_profiles(state: &ProxyState) {
    let config = state.config.read().await;
    let profiles: Vec<_> = config.enabled_profiles().into_iter().cloned().collect();
    drop(config);

    for profile in &profiles {
        let result = crate::config::profile::test_connectivity(profile).await;
        let status = match result {
            Ok(latency) => HealthStatus {
                healthy: true,
                latency_ms: Some(latency),
                last_check: Some(std::time::Instant::now()),
                error: None,
            },
            Err(e) => HealthStatus {
                healthy: false,
                latency_ms: None,
                last_check: Some(std::time::Instant::now()),
                error: Some(e.to_string()),
            },
        };

        let mut map = state.health_status.write().await;
        map.insert(profile.name.clone(), status);
    }
}
