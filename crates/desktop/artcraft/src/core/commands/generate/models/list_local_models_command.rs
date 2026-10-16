//! Tauri command: the models that run on the local ComfyUI server, in the shape of the ArtCraft
//! model listing so the prompt boxes treat them like any other model.

use serde_derive::Serialize;

use crate::core::commands::generate::comfyui::{minimax_h3_workflow, qwen_image_workflow, MINIMAX_H3_MODEL, QWEN_IMAGE_MODEL};
use crate::core::commands::generate::models::video::listing_video_model::ListingVideoModel;
use crate::core::commands::response::shorthand::ResponseOrErrorMessage;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;

const LOCAL_DESCRIPTION: &str = "Runs on your ComfyUI";

#[derive(Serialize)]
pub struct ListLocalModelsResponse {
  pub video_models: Vec<ListingVideoModel>,
  pub image_models: Vec<ListingImageModel>,
}

impl SerializeMarker for ListLocalModelsResponse {}

/// An image model in the shape of the ArtCraft listing (`ListingImageModel` in the frontend).
#[derive(Serialize)]
pub struct ListingImageModel {
  pub model: String,
  pub full_name: String,
  pub model_creator: Option<&'static str>,
  pub text_prompt_supported: bool,
  pub batch_size_options: Vec<u32>,
  pub batch_size_default: u32,
  pub batch_size_max: u32,
  /// ArtCraft `CommonAspectRatio` values.
  pub aspect_ratio_options: Vec<String>,
  pub aspect_ratio_default: Option<String>,
  pub image_refs_supported: bool,
}

#[tauri::command]
pub async fn list_local_models_command() -> ResponseOrErrorMessage<ListLocalModelsResponse> {
  Ok(ListLocalModelsResponse {
    video_models: vec![minimax_h3()],
    image_models: vec![qwen_image()],
  }.into())
}

fn minimax_h3() -> ListingVideoModel {
  ListingVideoModel {
    model: MINIMAX_H3_MODEL.to_string(),
    full_name: "MiniMax H3 · Local".to_string(),
    model_creator: Some("hailuo"),
    extra_info_short: LOCAL_DESCRIPTION,
    text_to_video_supported: true,
    starting_keyframe_supported: true,
    ending_keyframe_supported: true,
    image_references_supported: false,
    image_references_max: 0,
    show_generate_with_sound_toggle: true,
    duration_seconds_options: minimax_h3_workflow::DURATIONS.to_vec(),
    duration_seconds_default: Some(minimax_h3_workflow::DEFAULT_DURATION),
    resolution_options: minimax_h3_workflow::RESOLUTIONS.iter().map(|value| value.to_string()).collect(),
    resolution_default: Some(minimax_h3_workflow::RESOLUTIONS[0].to_string()),
    aspect_ratio_options: minimax_h3_workflow::ASPECT_RATIOS.iter().map(|value| value.to_string()).collect(),
    aspect_ratio_default: Some(minimax_h3_workflow::ASPECT_RATIOS[0].to_string()),
  }
}

/// One image per run: several would need a library batch, and each takes minutes on one GPU.
fn qwen_image() -> ListingImageModel {
  ListingImageModel {
    model: QWEN_IMAGE_MODEL.to_string(),
    full_name: "Qwen-Image 2.1 · Local".to_string(),
    model_creator: Some("alibaba"),
    text_prompt_supported: true,
    batch_size_options: vec![1],
    batch_size_default: 1,
    batch_size_max: 1,
    aspect_ratio_options: qwen_image_workflow::ASPECT_RATIOS.iter().map(|(name, _, _)| name.to_string()).collect(),
    aspect_ratio_default: Some(qwen_image_workflow::ASPECT_RATIOS[0].0.to_string()),
    image_refs_supported: false,
  }
}
