use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::ComfyuiError;

pub(crate) async fn read_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, ComfyuiError> {
  let body = check_status(response).await?.text().await?;
  serde_json::from_str(&body).map_err(ComfyuiError::Decode)
}

/// Turns a non-success status into `ComfyuiError::Api`.
pub(crate) async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, ComfyuiError> {
  let status = response.status();
  if status.is_success() {
    return Ok(response);
  }
  let body = response.text().await.unwrap_or_default();
  Err(ComfyuiError::Api { status: status.as_u16(), message: error_message(&body) })
}

/// A rejected workflow looks like `{"error": {"message": "...", "details": "..."}, "node_errors":
/// {"6": {"errors": [{"message": "...", "details": "..."}]}}}`. Joins whatever is present.
fn error_message(body: &str) -> String {
  let Ok(parsed) = serde_json::from_str::<Value>(body) else {
    return body.to_string();
  };
  let mut parts = Vec::new();
  let error = &parsed["error"];
  for field in ["message", "details"] {
    if let Some(text) = error[field].as_str().filter(|text| !text.is_empty()) {
      parts.push(text.to_string());
    }
  }
  if let Some(node_errors) = parsed["node_errors"].as_object() {
    for (node_id, node) in node_errors {
      for error in node["errors"].as_array().into_iter().flatten() {
        let message = error["message"].as_str().unwrap_or_default();
        let details = error["details"].as_str().unwrap_or_default();
        parts.push(format!("node {}: {} {}", node_id, message, details).trim_end().to_string());
      }
    }
  }
  if parts.is_empty() { body.to_string() } else { parts.join("; ") }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn error_message_joins_prompt_and_node_errors() {
    let body = r#"{
      "error": {"type": "prompt_outputs_failed_validation", "message": "Prompt outputs failed validation", "details": ""},
      "node_errors": {"1": {"errors": [{"message": "Value not in list", "details": "unet_name: 'x.gguf' not in []"}]}}
    }"#;
    assert_eq!(error_message(body), "Prompt outputs failed validation; node 1: Value not in list unet_name: 'x.gguf' not in []");
  }

  #[test]
  fn error_message_falls_back_to_raw_body() {
    assert_eq!(error_message("Internal Server Error"), "Internal Server Error");
  }
}
