use crate::error::OpenrouterError;
use crate::http::{authorized_get, read_json};
use crate::videos::video_job::VideoJob;

/// Polls a job through the `polling_url` it was created with.
pub async fn get_video(api_key: &str, polling_url: &str) -> Result<VideoJob, OpenrouterError> {
  read_json(authorized_get(api_key, polling_url).await?).await
}
