//! MiniMax H3 (pruned first/last-frame model, 8-step turbo LoRA) text- and image-to-video.

use serde_json::{json, Value};

const WORKFLOW: &str = include_str!("../../../../../comfyui_workflows/minimax_h3_fl2va.json");

/// Seconds offered in the prompt box. H3 was trained on about 5 to 15 seconds.
pub const DURATIONS: [u32; 3] = [5, 8, 10];
pub const DEFAULT_DURATION: u32 = 5;
/// Shown as-is; the prompt box sends them back as `four_eighty_p` / `seven_twenty_p`.
pub const RESOLUTIONS: [&str; 2] = ["480p", "720p"];
pub const ASPECT_RATIOS: [&str; 3] = ["wide_sixteen_by_nine", "tall_nine_by_sixteen", "square"];

pub struct MinimaxH3Input<'a> {
  pub prompt: &'a str,
  /// ArtCraft `CommonAspectRatio` value.
  pub aspect_ratio: Option<&'a str>,
  /// ArtCraft `CommonResolution` value.
  pub resolution: Option<&'a str>,
  pub duration_seconds: Option<u32>,
  pub generate_audio: bool,
  pub seed: u64,
  /// Image names in ComfyUI's input folder.
  pub first_frame: Option<&'a str>,
  pub last_frame: Option<&'a str>,
}

pub fn build_workflow(input: &MinimaxH3Input) -> Value {
  let mut workflow: Value = serde_json::from_str(WORKFLOW).expect("bundled H3 workflow is valid JSON");
  let (width, height) = dimensions(input.aspect_ratio, input.resolution);
  let video = &mut workflow["6"]["inputs"];
  video["prompt"] = json!(input.prompt);
  video["width"] = json!(width);
  video["height"] = json!(height);
  video["length"] = json!(frame_count(input.duration_seconds.unwrap_or(DEFAULT_DURATION)));
  workflow["7"]["inputs"]["noise_seed"] = json!(input.seed);

  for (node_id, frame, input_name) in [("20", input.first_frame, "first_frame"), ("21", input.last_frame, "last_frame")] {
    if let Some(image) = frame {
      workflow[node_id] = json!({ "class_type": "LoadImage", "inputs": { "image": image } });
      workflow["6"]["inputs"][input_name] = json!([node_id, 0]);
    }
  }

  if !input.generate_audio {
    workflow.as_object_mut().expect("workflow is an object").remove("13");
    workflow["14"]["inputs"].as_object_mut().expect("node inputs are an object").remove("audio");
  }
  workflow
}

/// Sizes divisible by 32, as the model requires: 0.4 megapixels at 480p, 0.9 at 720p.
fn dimensions(aspect_ratio: Option<&str>, resolution: Option<&str>) -> (u32, u32) {
  let is_720p = matches!(resolution, Some("seven_twenty_p" | "720p"));
  match (aspect_ratio.unwrap_or(ASPECT_RATIOS[0]), is_720p) {
    ("tall_nine_by_sixteen", false) => (480, 864),
    ("square", false) => (640, 640),
    (_, false) => (864, 480),
    ("tall_nine_by_sixteen", true) => (704, 1280),
    ("square", true) => (960, 960),
    (_, true) => (1280, 704),
  }
}

/// Seconds to frames at 24 fps, snapped up to H3's 17k+5 grid like the official template does.
fn frame_count(seconds: u32) -> u32 {
  let frames = (seconds * 24).max(5);
  frames + (5 + 17 - frames % 17) % 17
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn frame_counts_match_the_official_template() {
    assert_eq!(frame_count(5), 124);
    assert_eq!(frame_count(8), 192);
    assert_eq!(frame_count(10), 243);
  }

  #[test]
  fn text_to_video_fills_prompt_size_length_and_seed() {
    let workflow = build_workflow(&MinimaxH3Input {
      prompt: "a red panda on a mossy log",
      aspect_ratio: Some("tall_nine_by_sixteen"),
      resolution: Some("seven_twenty_p"),
      duration_seconds: Some(8),
      generate_audio: true,
      seed: 7,
      first_frame: None,
      last_frame: None,
    });
    let video = &workflow["6"]["inputs"];
    assert_eq!(video["prompt"], "a red panda on a mossy log");
    assert_eq!((video["width"].as_u64(), video["height"].as_u64()), (Some(704), Some(1280)));
    assert_eq!(video["length"], 192);
    assert_eq!(workflow["7"]["inputs"]["noise_seed"], 7);
    assert!(video.get("first_frame").is_none());
    assert_eq!(workflow["14"]["inputs"]["audio"], json!(["13", 0]));
  }

  #[test]
  fn frames_add_load_image_nodes_and_silence_drops_audio() {
    let workflow = build_workflow(&MinimaxH3Input {
      prompt: "",
      aspect_ratio: None,
      resolution: None,
      duration_seconds: None,
      generate_audio: false,
      seed: 1,
      first_frame: Some("artcraft_first.png"),
      last_frame: Some("artcraft_last.png"),
    });
    assert_eq!(workflow["20"]["inputs"]["image"], "artcraft_first.png");
    assert_eq!(workflow["6"]["inputs"]["first_frame"], json!(["20", 0]));
    assert_eq!(workflow["6"]["inputs"]["last_frame"], json!(["21", 0]));
    assert_eq!((workflow["6"]["inputs"]["width"].as_u64(), workflow["6"]["inputs"]["length"].as_u64()), (Some(864), Some(124)));
    assert!(workflow.get("13").is_none());
    assert!(workflow["14"]["inputs"].get("audio").is_none());
  }
}
