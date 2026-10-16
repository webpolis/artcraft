use std::error::Error;
use std::fmt::{Display, Formatter};

/// Every way a ComfyUI call can fail.
#[derive(Debug)]
pub enum ComfyuiError {
  /// No HTTP response arrived; usually ComfyUI isn't running.
  Http(reqwest::Error),
  /// ComfyUI answered with a non-success status, e.g. a workflow that failed validation.
  Api { status: u16, message: String },
  /// The success body didn't have the expected shape.
  Decode(serde_json::Error),
}

impl Error for ComfyuiError {}

impl Display for ComfyuiError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl From<reqwest::Error> for ComfyuiError {
  fn from(error: reqwest::Error) -> Self {
    Self::Http(error)
  }
}
