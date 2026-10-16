use serde_derive::Deserialize;

/// A video generation job, as returned on submit and on every poll.
#[derive(Clone, Debug, Deserialize)]
pub struct VideoJob {
  pub id: String,
  pub polling_url: String,
  pub status: VideoJobStatus,
  /// Content URLs, one per output. They need the API key. Present once `completed`.
  #[serde(default)]
  pub unsigned_urls: Vec<String>,
  /// Why the job failed. Present when `failed`.
  pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoJobStatus {
  Pending,
  InProgress,
  Completed,
  Failed,
  Cancelled,
  Expired,
  /// A status added after this client was written; treated as still running.
  #[serde(other)]
  Unknown,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_submit_response() {
    let job: VideoJob = serde_json::from_str(
      r#"{"id":"abc123","polling_url":"https://openrouter.ai/api/v1/videos/abc123","status":"pending"}"#,
    ).unwrap();
    assert_eq!(job.id, "abc123");
    assert_eq!(job.status, VideoJobStatus::Pending);
    assert!(job.unsigned_urls.is_empty());
    assert_eq!(job.error, None);
  }

  #[test]
  fn parses_completed_poll_with_extra_fields() {
    let job: VideoJob = serde_json::from_str(r#"{
      "id": "abc123",
      "generation_id": "gen-1234567890-abcdef",
      "polling_url": "https://openrouter.ai/api/v1/videos/abc123",
      "status": "completed",
      "unsigned_urls": ["https://openrouter.ai/api/v1/videos/abc123/content?index=0"],
      "usage": {"cost": 0.25, "is_byok": false}
    }"#).unwrap();
    assert_eq!(job.status, VideoJobStatus::Completed);
    assert_eq!(job.unsigned_urls, vec!["https://openrouter.ai/api/v1/videos/abc123/content?index=0"]);
  }

  #[test]
  fn parses_failure_and_unknown_status() {
    let failed: VideoJob = serde_json::from_str(
      r#"{"id":"a","polling_url":"https://openrouter.ai/api/v1/videos/a","status":"failed","error":"Content policy"}"#,
    ).unwrap();
    assert_eq!(failed.status, VideoJobStatus::Failed);
    assert_eq!(failed.error.as_deref(), Some("Content policy"));

    let unknown: VideoJob = serde_json::from_str(
      r#"{"id":"a","polling_url":"https://openrouter.ai/api/v1/videos/a","status":"upscaling"}"#,
    ).unwrap();
    assert_eq!(unknown.status, VideoJobStatus::Unknown);
  }
}
