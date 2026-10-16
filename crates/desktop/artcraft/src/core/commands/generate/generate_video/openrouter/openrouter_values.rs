//! OpenRouter and ArtCraft name the same options differently. The prompt box speaks ArtCraft's
//! common enums (`wide_sixteen_by_nine`, `ten_eighty_p`); OpenRouter speaks `16:9` and `1080p`.
//! Values missing from these tables pass through unchanged.

/// (OpenRouter, ArtCraft `CommonAspectRatio`)
const ASPECT_RATIOS: [(&str, &str); 9] = [
  ("1:1", "square"),
  ("16:9", "wide_sixteen_by_nine"),
  ("9:16", "tall_nine_by_sixteen"),
  ("4:3", "wide_four_by_three"),
  ("3:4", "tall_three_by_four"),
  ("3:2", "wide_three_by_two"),
  ("2:3", "tall_two_by_three"),
  ("21:9", "wide_twenty_one_by_nine"),
  ("9:21", "tall_nine_by_twenty_one"),
];

/// (OpenRouter, ArtCraft `CommonResolution`)
const RESOLUTIONS: [(&str, &str); 6] = [
  ("480p", "four_eighty_p"),
  ("720p", "seven_twenty_p"),
  ("1080p", "ten_eighty_p"),
  ("1K", "one_k"),
  ("2K", "two_k"),
  ("4K", "four_k"),
];

/// (OpenRouter model id prefix, ArtCraft model creator)
const CREATORS: [(&str, &str); 9] = [
  ("alibaba/", "alibaba"),
  ("black-forest-labs/", "black_forest_labs"),
  ("bytedance/", "bytedance"),
  ("google/", "google"),
  ("kwaivgi/", "kling"),
  ("minimax/", "hailuo"),
  ("openai/", "open_ai"),
  ("runway/", "runway"),
  ("x-ai/", "grok"),
];

pub fn artcraft_aspect_ratio(openrouter_value: &str) -> String {
  ASPECT_RATIOS.iter()
    .find(|(openrouter, _)| *openrouter == openrouter_value)
    .map_or(openrouter_value, |(_, artcraft)| artcraft)
    .to_string()
}

pub fn openrouter_aspect_ratio(artcraft_value: &str) -> String {
  ASPECT_RATIOS.iter()
    .find(|(_, artcraft)| *artcraft == artcraft_value)
    .map_or(artcraft_value, |(openrouter, _)| openrouter)
    .to_string()
}

pub fn openrouter_resolution(artcraft_value: &str) -> String {
  RESOLUTIONS.iter()
    .find(|(_, artcraft)| *artcraft == artcraft_value)
    .map_or(artcraft_value, |(openrouter, _)| openrouter)
    .to_string()
}

/// The ArtCraft creator for a model id like `kwaivgi/kling-v3.0-pro`, for the picker's icon.
pub fn artcraft_creator(model_id: &str) -> Option<&'static str> {
  CREATORS.iter()
    .find(|(prefix, _)| model_id.starts_with(prefix))
    .map(|(_, creator)| *creator)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn aspect_ratios_round_trip() {
    for (openrouter, artcraft) in ASPECT_RATIOS {
      assert_eq!(artcraft_aspect_ratio(openrouter), artcraft);
      assert_eq!(openrouter_aspect_ratio(artcraft), openrouter);
    }
  }

  #[test]
  fn unknown_values_pass_through() {
    assert_eq!(artcraft_aspect_ratio("5:4"), "5:4");
    assert_eq!(openrouter_aspect_ratio("5:4"), "5:4");
    assert_eq!(openrouter_resolution("768p"), "768p");
  }

  #[test]
  fn resolutions_map_from_prompt_box_values() {
    assert_eq!(openrouter_resolution("ten_eighty_p"), "1080p");
    assert_eq!(openrouter_resolution("four_k"), "4K");
  }

  #[test]
  fn creators_come_from_model_id_prefix() {
    assert_eq!(artcraft_creator("kwaivgi/kling-v3.0-pro"), Some("kling"));
    assert_eq!(artcraft_creator("google/veo-3.1"), Some("google"));
    assert_eq!(artcraft_creator("heygen/heygen-video-1"), None);
  }
}
