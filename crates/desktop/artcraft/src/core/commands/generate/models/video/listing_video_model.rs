use serde_derive::Serialize;

/// A video model described in the shape of the ArtCraft model listing (`ListingVideoModel` in the
/// frontend), for providers whose catalog doesn't come from ArtCraft's servers.
#[derive(Serialize)]
pub struct ListingVideoModel {
  pub model: String,
  pub full_name: String,
  pub model_creator: Option<&'static str>,
  pub extra_info_short: &'static str,
  pub text_to_video_supported: bool,
  pub starting_keyframe_supported: bool,
  pub ending_keyframe_supported: bool,
  pub image_references_supported: bool,
  pub image_references_max: u32,
  pub show_generate_with_sound_toggle: bool,
  pub duration_seconds_options: Vec<u32>,
  pub duration_seconds_default: Option<u32>,
  /// Display values such as `720p` or `4K`; the prompt box maps known ones itself.
  pub resolution_options: Vec<String>,
  pub resolution_default: Option<String>,
  /// ArtCraft `CommonAspectRatio` values.
  pub aspect_ratio_options: Vec<String>,
  pub aspect_ratio_default: Option<String>,
}
