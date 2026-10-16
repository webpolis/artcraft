use serde::de::DeserializeOwned;
use serde_derive::Deserialize;

use crate::error::OpenrouterError;

/// Only URLs under this prefix get the API key.
const TRUSTED_URL_PREFIX: &str = "https://openrouter.ai/";

#[derive(Deserialize)]
struct ErrorBody {
  error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
  message: String,
}

/// GET a URL that came back in an OpenRouter response (polling or content URL), with the key.
pub(crate) async fn authorized_get(api_key: &str, url: &str) -> Result<reqwest::Response, OpenrouterError> {
  if !url.starts_with(TRUSTED_URL_PREFIX) {
    return Err(OpenrouterError::UntrustedUrl(url.to_string()));
  }
  let response = reqwest::Client::new().get(url).bearer_auth(api_key).send().await?;
  check_status(response).await
}

pub(crate) async fn read_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, OpenrouterError> {
  let body = check_status(response).await?.text().await?;
  serde_json::from_str(&body).map_err(OpenrouterError::Decode)
}

/// Turns a non-success status into `OpenrouterError::Api`.
pub(crate) async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, OpenrouterError> {
  let status = response.status();
  if status.is_success() {
    return Ok(response);
  }
  let body = response.text().await.unwrap_or_default();
  Err(OpenrouterError::Api { status: status.as_u16(), message: error_message(&body) })
}

/// OpenRouter errors look like `{"error": {"code": 402, "message": "Insufficient credits..."}}`.
fn error_message(body: &str) -> String {
  serde_json::from_str::<ErrorBody>(body)
    .map(|parsed| parsed.error.message)
    .unwrap_or_else(|_| body.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn error_message_reads_openrouter_error_body() {
    let body = r#"{"error":{"code":402,"message":"Insufficient credits"}}"#;
    assert_eq!(error_message(body), "Insufficient credits");
  }

  #[test]
  fn error_message_falls_back_to_raw_body() {
    assert_eq!(error_message("Bad Gateway"), "Bad Gateway");
  }

  #[tokio::test]
  async fn authorized_get_refuses_other_hosts() {
    let result = authorized_get("sk-or-test", "https://example.com/videos/abc").await;
    assert!(matches!(result, Err(OpenrouterError::UntrustedUrl(_))));
  }
}
