use artcraft_api_defs::utils::media_links_to_thumbnail_template::media_links_to_thumbnail_template;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::endpoints::media_files::get_media_file::get_media_file;
use artcraft_client::endpoints::media_files::upload_image_media_file_from_file::{
  upload_image_media_file_from_file, UploadImageFromFileArgs,
};
use artcraft_client::endpoints::media_files::upload_video_media_file_from_file::{
  upload_video_media_file_from_file, UploadVideoFromFileArgs,
};
use artcraft_client::error::storyteller_error::StorytellerError;
use artcraft_client::utils::api_host::ApiHost;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use log::{error, info, warn};
use sqlite_tasks::queries::task::Task;
use sqlite_tasks::queries::update_successful_task_status_with_metadata::{
  update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs,
};
use std::path::Path;
use tauri::AppHandle;
use tokens::tokens::media_files::MediaFileToken;
use tokens::tokens::prompts::PromptToken;
use uuid_utils::uuid::generate_random_uuid;

use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::events::notify_frontend_of_completion::{
  notify_frontend_of_completion, CompletionData,
};
use crate::core::utils::auto_download::{auto_download_task_urls, clear_auto_download_checkpoint};

/// A finished third-party result, downloaded but not yet in the library.
pub struct GeneratedFile {
  pub bytes: Vec<u8>,
  /// File extension without the dot, e.g. `mp4` or `png`.
  pub extension: &'static str,
  pub media_class: TaskMediaFileClass,
}

/// Uploads a finished result to the ArtCraft library and marks its task done. An error before the
/// upload leaves the task pending for the next polling pass; after it, everything fails open,
/// because retrying would upload the file twice.
pub async fn save_generated_file(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  creds: &StorytellerCredentialSet,
  task: &Task,
  file: GeneratedFile,
) -> Result<(), Box<dyn std::error::Error>> {
  let host = &app_env_configs.storyteller_host;
  let temp_path = app_data_root.temp_dir().path().join(format!("generated_{}.{}", generate_random_uuid(), file.extension));
  std::fs::write(&temp_path, &file.bytes)?;

  let maybe_prompt_token = task.prompt_token.as_ref().map(|token| PromptToken::new_from_str(token));
  let upload_result = upload(host, creds, &temp_path, maybe_prompt_token.as_ref(), file.media_class).await;
  if let Err(err) = std::fs::remove_file(&temp_path) {
    warn!("[ThirdPartyComplete] Could not remove {:?}: {}", temp_path, err);
  }
  let media_file_token = upload_result?;
  info!("[ThirdPartyComplete] Task {} uploaded as {:?}", task.id.as_str(), media_file_token);

  let (maybe_cdn_url, maybe_thumbnail_url_template) = match get_media_file(host, &media_file_token).await {
    Ok(response) => {
      let links = &response.media_file.media_links;
      (Some(links.cdn_url.clone()), media_links_to_thumbnail_template(links).map(|template| template.to_string()))
    },
    Err(err) => {
      error!("[ThirdPartyComplete] Failed to look up media file after upload: {:?} (failing open)", err);
      (None, None)
    },
  };

  if let Some(cdn_url) = maybe_cdn_url.as_ref() {
    if let Err(err) = auto_download_task_urls(app_handle, task, &[cdn_url.clone()]).await {
      warn!("[ThirdPartyComplete] Automatic download failed for task {}: {:?}", task.id.as_str(), err);
    }
  }

  let maybe_cdn_url_str = maybe_cdn_url.as_ref().map(|url| url.to_string());
  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
    db: task_database.get_connection(),
    task_id: &task.id,
    maybe_batch_token: None,
    maybe_primary_media_file_token: Some(&media_file_token),
    maybe_primary_media_file_class: Some(file.media_class),
    maybe_primary_media_file_cdn_url: maybe_cdn_url_str.as_deref(),
    maybe_primary_media_file_thumbnail_url_template: maybe_thumbnail_url_template.as_deref(),
  }).await?;

  if updated {
    clear_auto_download_checkpoint(app_handle, task);
    let completion = CompletionData {
      primary_media_file_token: media_file_token,
      maybe_cdn_url,
      maybe_thumbnail_url_template,
      maybe_batch_token: None,
      media_class: file.media_class,
    };
    notify_frontend_of_completion(app_handle, host, Some(creds), task, &completion).await;
  }

  info!("[ThirdPartyComplete] Task {} fully handled", task.id.as_str());
  Ok(())
}

async fn upload(
  host: &ApiHost,
  creds: &StorytellerCredentialSet,
  path: &Path,
  maybe_prompt_token: Option<&PromptToken>,
  media_class: TaskMediaFileClass,
) -> Result<MediaFileToken, StorytellerError> {
  // Without a provider the server files the result as a user upload, which My Library hides.
  // The server knows neither OpenRouter nor ComfyUI, so name the app that generated it.
  let provider = Some(GenerationProvider::Artcraft);
  let token = match media_class {
    TaskMediaFileClass::Video => upload_video_media_file_from_file(UploadVideoFromFileArgs {
      api_host: host,
      maybe_creds: Some(creds),
      path,
      maybe_prompt_token,
      maybe_generation_provider: provider,
    }).await?.media_file_token,
    _ => upload_image_media_file_from_file(UploadImageFromFileArgs {
      api_host: host,
      maybe_creds: Some(creds),
      path,
      is_intermediate_system_file: false,
      maybe_prompt_token,
      maybe_batch_token: None,
      maybe_generation_provider: provider,
    }).await?.media_file_token,
  };
  Ok(token)
}
