use crate::error::OpenrouterError;
use crate::http::authorized_get;

/// Downloads one finished output (an entry of `VideoJob::unsigned_urls`) as MP4 bytes.
pub async fn download_video(api_key: &str, content_url: &str) -> Result<Vec<u8>, OpenrouterError> {
  let bytes = authorized_get(api_key, content_url).await?.bytes().await?;
  Ok(bytes.into())
}
