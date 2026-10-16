//! Qwen-Image 2.1 (GGUF) text-to-image.

use serde_json::{json, Value};

const WORKFLOW: &str = include_str!("../../../../../comfyui_workflows/qwen_image_2_1.json");

/// ArtCraft `CommonAspectRatio` values with Qwen-Image's recommended sizes for each.
pub const ASPECT_RATIOS: [(&str, u32, u32); 7] = [
  ("square", 1328, 1328),
  ("wide_sixteen_by_nine", 1664, 928),
  ("tall_nine_by_sixteen", 928, 1664),
  ("wide_four_by_three", 1472, 1104),
  ("tall_three_by_four", 1104, 1472),
  ("wide_three_by_two", 1584, 1056),
  ("tall_two_by_three", 1056, 1584),
];

pub struct QwenImageInput<'a> {
  pub prompt: &'a str,
  pub aspect_ratio: Option<&'a str>,
  pub seed: u64,
}

pub fn build_workflow(input: &QwenImageInput) -> Value {
  let mut workflow: Value = serde_json::from_str(WORKFLOW).expect("bundled Qwen-Image workflow is valid JSON");
  let (_, width, height) = ASPECT_RATIOS.iter()
    .find(|(name, _, _)| Some(*name) == input.aspect_ratio)
    .unwrap_or(&ASPECT_RATIOS[0]);
  workflow["4"]["inputs"]["text"] = json!(input.prompt);
  workflow["6"]["inputs"]["width"] = json!(width);
  workflow["6"]["inputs"]["height"] = json!(height);
  workflow["7"]["inputs"]["seed"] = json!(input.seed);
  workflow
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fills_prompt_size_and_seed() {
    let workflow = build_workflow(&QwenImageInput { prompt: "a watercolor fox", aspect_ratio: Some("wide_sixteen_by_nine"), seed: 9 });
    assert_eq!(workflow["4"]["inputs"]["text"], "a watercolor fox");
    assert_eq!((workflow["6"]["inputs"]["width"].as_u64(), workflow["6"]["inputs"]["height"].as_u64()), (Some(1664), Some(928)));
    assert_eq!(workflow["7"]["inputs"]["seed"], 9);
  }

  #[test]
  fn unknown_aspect_ratio_falls_back_to_square() {
    let workflow = build_workflow(&QwenImageInput { prompt: "", aspect_ratio: Some("auto"), seed: 1 });
    assert_eq!(workflow["6"]["inputs"]["width"], 1328);
  }
}
