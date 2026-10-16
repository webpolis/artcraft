//! Tauri command: list OpenRouter's video models in the shape of the ArtCraft video model listing
//! (`ListingVideoModel` in the frontend), so the prompt box treats them like any other model.

use log::{info, warn};
use openrouter_client::videos::list_video_models::{list_video_models, VideoModelInfo};
use serde_derive::Serialize;

use crate::core::commands::generate::generate_video::openrouter::openrouter_values::{artcraft_aspect_ratio, artcraft_creator};
use crate::core::commands::generate::models::video::listing_video_model::ListingVideoModel;
use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;

/// OpenRouter takes image references for every model. Veo 3.1 accepts at most 3, so all models use that cap.
const MAX_REFERENCE_IMAGES: u32 = 3;

#[derive(Serialize)]
pub struct ListOpenrouterVideoModelsResponse {
  pub models: Vec<ListingVideoModel>,
}

impl SerializeMarker for ListOpenrouterVideoModelsResponse {}

#[tauri::command]
pub async fn list_openrouter_video_models_command() -> ResponseOrErrorMessage<ListOpenrouterVideoModelsResponse> {
  let catalog = list_video_models().await.map_err(|err| {
    warn!("Could not list OpenRouter video models: {:?}", err);
    "Could not load OpenRouter video models"
  })?;

  let models: Vec<_> = catalog.into_iter()
    // Edit, upscale and avatar models have no durations: they need video or audio input.
    .filter(|model| model.supported_durations.is_some())
    .map(to_listing_model)
    .collect();

  info!("Listed {} OpenRouter video models", models.len());
  Ok(ListOpenrouterVideoModelsResponse { models }.into())
}

fn to_listing_model(model: VideoModelInfo) -> ListingVideoModel {
  let frames = model.supported_frame_images.unwrap_or_default();
  let durations = model.supported_durations.unwrap_or_default();
  let resolutions = model.supported_resolutions.unwrap_or_default();
  let aspect_ratios: Vec<String> = model.supported_aspect_ratios.unwrap_or_default()
    .iter()
    .map(|value| artcraft_aspect_ratio(value))
    .collect();

  // "Google: Veo 3.1" -> "Veo 3.1"; the picker shows the creator's icon.
  let name = model.name.split_once(": ").map_or(model.name.as_str(), |(_, name)| name);

  ListingVideoModel {
    full_name: format!("{} · OpenRouter", name),
    model_creator: artcraft_creator(&model.id),
    extra_info_short: "Billed to your OpenRouter account",
    text_to_video_supported: true,
    starting_keyframe_supported: frames.iter().any(|frame| frame == "first_frame"),
    ending_keyframe_supported: frames.iter().any(|frame| frame == "last_frame"),
    image_references_supported: true,
    image_references_max: MAX_REFERENCE_IMAGES,
    show_generate_with_sound_toggle: model.generate_audio.unwrap_or(false),
    duration_seconds_default: preferred(&durations, &5).or_else(|| durations.first().copied()),
    resolution_default: preferred(&resolutions, &"1080p".to_string()).or_else(|| resolutions.last().cloned()),
    aspect_ratio_default: preferred(&aspect_ratios, &"wide_sixteen_by_nine".to_string()).or_else(|| aspect_ratios.first().cloned()),
    duration_seconds_options: durations,
    resolution_options: resolutions,
    aspect_ratio_options: aspect_ratios,
    model: model.id,
  }
}

fn preferred<T: PartialEq + Clone>(options: &[T], wanted: &T) -> Option<T> {
  options.contains(wanted).then(|| wanted.clone())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_catalog_entry_to_listing_shape() {
    let listing = to_listing_model(VideoModelInfo {
      id: "google/veo-3.1".to_string(),
      name: "Google: Veo 3.1".to_string(),
      supported_durations: Some(vec![4, 6, 8]),
      supported_resolutions: Some(vec!["720p".to_string(), "1080p".to_string(), "4K".to_string()]),
      supported_aspect_ratios: Some(vec!["16:9".to_string(), "9:16".to_string()]),
      supported_frame_images: Some(vec!["first_frame".to_string()]),
      generate_audio: Some(true),
    });

    assert_eq!(listing.model, "google/veo-3.1");
    assert_eq!(listing.full_name, "Veo 3.1 · OpenRouter");
    assert_eq!(listing.model_creator, Some("google"));
    assert!(listing.starting_keyframe_supported);
    assert!(!listing.ending_keyframe_supported);
    assert!(listing.show_generate_with_sound_toggle);
    assert_eq!(listing.duration_seconds_default, Some(4));
    assert_eq!(listing.resolution_default.as_deref(), Some("1080p"));
    assert_eq!(listing.aspect_ratio_options, vec!["wide_sixteen_by_nine", "tall_nine_by_sixteen"]);
    assert_eq!(listing.aspect_ratio_default.as_deref(), Some("wide_sixteen_by_nine"));
  }

  #[test]
  fn defaults_fall_back_to_offered_options() {
    let listing = to_listing_model(VideoModelInfo {
      id: "minimax/hailuo-3".to_string(),
      name: "MiniMax: Hailuo 3".to_string(),
      supported_durations: Some(vec![5, 6, 10]),
      supported_resolutions: Some(vec!["2K".to_string()]),
      supported_aspect_ratios: Some(vec!["1:1".to_string()]),
      supported_frame_images: None,
      generate_audio: None,
    });

    assert_eq!(listing.duration_seconds_default, Some(5));
    assert_eq!(listing.resolution_default.as_deref(), Some("2K"));
    assert_eq!(listing.aspect_ratio_default.as_deref(), Some("square"));
    assert!(!listing.starting_keyframe_supported);
    assert!(!listing.show_generate_with_sound_toggle);
  }
}
