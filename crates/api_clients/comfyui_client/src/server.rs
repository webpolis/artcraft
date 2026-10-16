use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Whether a ComfyUI server answers at `host`.
pub async fn is_running(host: &str) -> bool {
  reqwest::Client::new()
    .get(format!("{}/system_stats", host))
    .timeout(PROBE_TIMEOUT)
    .send()
    .await
    .is_ok_and(|response| response.status().is_success())
}
