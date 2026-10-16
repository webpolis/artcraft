use log::info;
use serde_derive::Serialize;

use crate::error::OpenrouterError;
use crate::http::read_json;
use crate::videos::video_job::VideoJob;

const VIDEOS_URL: &str = "https://openrouter.ai/api/v1/videos";

/// Body of `POST /api/v1/videos`. Unset options fall back to the model's defaults.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CreateVideoRequest {
  /// Model slug, e.g. `google/veo-3.1`.
  pub model: String,
  pub prompt: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub duration: Option<u32>,
  /// e.g. `720p`, `1080p`, `4K`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolution: Option<String>,
  /// e.g. `16:9`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub aspect_ratio: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub generate_audio: Option<bool>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub frame_images: Vec<FrameImage>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub input_references: Vec<ImageInput>,
}

/// An image pinned to the first or last frame (image-to-video).
#[derive(Clone, Debug, Serialize)]
pub struct FrameImage {
  #[serde(flatten)]
  pub image: ImageInput,
  pub frame_type: FrameType,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameType {
  FirstFrame,
  LastFrame,
}

/// An image passed by URL: `{"type": "image_url", "image_url": {"url": "..."}}`. The URL must be
/// publicly downloadable; providers don't follow cookies or bot checks.
#[derive(Clone, Debug, Serialize)]
pub struct ImageInput {
  #[serde(rename = "type")]
  kind: &'static str,
  image_url: ImageUrl,
}

#[derive(Clone, Debug, Serialize)]
struct ImageUrl {
  url: String,
}

impl ImageInput {
  pub fn from_url(url: impl Into<String>) -> Self {
    Self { kind: "image_url", image_url: ImageUrl { url: url.into() } }
  }
}

/// Starts a job and returns it as `pending`. Poll `polling_url` with `get_video` until it ends.
pub async fn create_video(api_key: &str, request: &CreateVideoRequest) -> Result<VideoJob, OpenrouterError> {
  info!(
    "OpenRouter create_video: model={}, duration={:?}, resolution={:?}, aspect_ratio={:?}, audio={:?}, frames={}, references={}",
    request.model, request.duration, request.resolution, request.aspect_ratio, request.generate_audio,
    request.frame_images.len(), request.input_references.len(),
  );
  let response = reqwest::Client::new().post(VIDEOS_URL).bearer_auth(api_key).json(request).send().await?;
  read_json(response).await
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn minimal_request_omits_unset_options() {
    let request = CreateVideoRequest {
      model: "google/veo-3.1".to_string(),
      prompt: "A flower blooming".to_string(),
      ..Default::default()
    };
    assert_eq!(serde_json::to_value(&request).unwrap(), json!({
      "model": "google/veo-3.1",
      "prompt": "A flower blooming",
    }));
  }

  #[test]
  fn full_request_matches_api_shape() {
    let request = CreateVideoRequest {
      model: "google/veo-3.1".to_string(),
      prompt: "Push in".to_string(),
      duration: Some(8),
      resolution: Some("1080p".to_string()),
      aspect_ratio: Some("16:9".to_string()),
      generate_audio: Some(false),
      frame_images: vec![
        FrameImage { image: ImageInput::from_url("https://cdn.example/first.png"), frame_type: FrameType::FirstFrame },
        FrameImage { image: ImageInput::from_url("https://cdn.example/last.png"), frame_type: FrameType::LastFrame },
      ],
      input_references: vec![ImageInput::from_url("https://cdn.example/style.png")],
    };
    assert_eq!(serde_json::to_value(&request).unwrap(), json!({
      "model": "google/veo-3.1",
      "prompt": "Push in",
      "duration": 8,
      "resolution": "1080p",
      "aspect_ratio": "16:9",
      "generate_audio": false,
      "frame_images": [
        {"type": "image_url", "image_url": {"url": "https://cdn.example/first.png"}, "frame_type": "first_frame"},
        {"type": "image_url", "image_url": {"url": "https://cdn.example/last.png"}, "frame_type": "last_frame"},
      ],
      "input_references": [
        {"type": "image_url", "image_url": {"url": "https://cdn.example/style.png"}},
      ],
    }));
  }
}
