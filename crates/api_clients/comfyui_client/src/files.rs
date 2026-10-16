use reqwest::multipart::{Form, Part};
use serde_derive::Deserialize;

use crate::error::ComfyuiError;
use crate::http::{check_status, read_json};

/// A file in one of ComfyUI's folders, as the history and upload routes describe it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct FileRef {
  pub filename: String,
  #[serde(default)]
  pub subfolder: String,
  /// `output`, `input` or `temp`.
  #[serde(rename = "type")]
  pub kind: String,
}

#[derive(Deserialize)]
struct UploadResponse {
  name: String,
}

/// Puts an image in ComfyUI's input folder and returns the name a `LoadImage` node takes.
pub async fn upload_image(host: &str, bytes: Vec<u8>, filename: &str) -> Result<String, ComfyuiError> {
  let form = Form::new()
    .part("image", Part::bytes(bytes).file_name(filename.to_string()))
    .text("type", "input")
    .text("overwrite", "true");
  let response = reqwest::Client::new().post(format!("{}/upload/image", host)).multipart(form).send().await?;
  Ok(read_json::<UploadResponse>(response).await?.name)
}

pub async fn download_file(host: &str, file: &FileRef) -> Result<Vec<u8>, ComfyuiError> {
  let response = reqwest::Client::new()
    .get(format!("{}/view", host))
    .query(&[("filename", &file.filename), ("subfolder", &file.subfolder), ("type", &file.kind)])
    .send()
    .await?;
  Ok(check_status(response).await?.bytes().await?.into())
}
