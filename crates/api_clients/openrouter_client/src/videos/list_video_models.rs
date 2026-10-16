use serde_derive::{Deserialize, Serialize};

use crate::error::OpenrouterError;
use crate::http::read_json;

const VIDEO_MODELS_URL: &str = "https://openrouter.ai/api/v1/videos/models";

/// One model from the catalog. Capabilities a model doesn't have are `null` or missing.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VideoModelInfo {
  /// Slug for `CreateVideoRequest::model`, e.g. `google/veo-3.1`.
  pub id: String,
  /// Display name, e.g. `Google: Veo 3.1`.
  pub name: String,
  pub supported_durations: Option<Vec<u32>>,
  pub supported_resolutions: Option<Vec<String>>,
  pub supported_aspect_ratios: Option<Vec<String>>,
  /// `first_frame` and/or `last_frame`.
  pub supported_frame_images: Option<Vec<String>>,
  /// Whether the model can generate sound with the video.
  pub generate_audio: Option<bool>,
}

#[derive(Deserialize)]
struct VideoModelsResponse {
  data: Vec<VideoModelInfo>,
}

/// Lists every video model. Needs no API key.
pub async fn list_video_models() -> Result<Vec<VideoModelInfo>, OpenrouterError> {
  let response = reqwest::Client::new().get(VIDEO_MODELS_URL).send().await?;
  Ok(read_json::<VideoModelsResponse>(response).await?.data)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_catalog_with_missing_capabilities() {
    let response: VideoModelsResponse = serde_json::from_str(r#"{"data": [
      {
        "id": "google/veo-3.1", "canonical_slug": "google/veo-3.1-20260320", "name": "Google: Veo 3.1",
        "supported_resolutions": ["720p", "1080p", "4K"], "supported_aspect_ratios": ["16:9", "9:16"],
        "supported_durations": [4, 6, 8], "supported_frame_images": ["first_frame", "last_frame"],
        "generate_audio": true, "pricing_skus": {"duration_seconds_with_audio": "0.40"}
      },
      {"id": "black-forest-labs/flux-video-edit", "name": "Flux Video Edit", "supported_durations": null}
    ]}"#).unwrap();

    let veo = &response.data[0];
    assert_eq!(veo.supported_durations, Some(vec![4, 6, 8]));
    assert_eq!(veo.supported_frame_images.as_deref(), Some(&["first_frame".to_string(), "last_frame".to_string()][..]));
    assert_eq!(veo.generate_audio, Some(true));

    let edit = &response.data[1];
    assert_eq!(edit.supported_durations, None);
    assert_eq!(edit.supported_resolutions, None);
  }
}
