use std::error::Error;
use std::fmt::{Display, Formatter};

/// Every way an OpenRouter call can fail.
#[derive(Debug)]
pub enum OpenrouterError {
  /// No HTTP response arrived (network, TLS, timeout).
  Http(reqwest::Error),
  /// OpenRouter answered with a non-success status. `message` comes from its error body.
  Api { status: u16, message: String },
  /// The success body didn't have the expected shape.
  Decode(serde_json::Error),
  /// We refused to send the API key to a host other than OpenRouter.
  UntrustedUrl(String),
}

impl Error for OpenrouterError {}

impl Display for OpenrouterError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl From<reqwest::Error> for OpenrouterError {
  fn from(error: reqwest::Error) -> Self {
    Self::Http(error)
  }
}
