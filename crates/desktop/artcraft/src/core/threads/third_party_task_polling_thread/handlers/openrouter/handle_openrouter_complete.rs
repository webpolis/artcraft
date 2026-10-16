use artcraft_api_defs::utils::media_links_to_thumbnail_template::media_links_to_thumbnail_template;
use artcraft_client::endpoints::media_files::get_media_file::get_media_file;
use artcraft_client::endpoints::media_files::upload_video_media_file_from_file::{
  upload_video_media_file_from_file, UploadVideoFromFileArgs,
};
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use log::{error, info, warn};
use openrouter_client::videos::download_video::download_video;
use openrouter_client::videos::video_job::VideoJob;
use sqlite_tasks::queries::task::Task;
use sqlite_tasks::queries::update_successful_task_status_with_metadata::{
  update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs,
};
use tauri::AppHandle;
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
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

/// Downloads the finished video, uploads it to the ArtCraft library and marks the task done.
/// Any failure before the upload leaves the task pending, so the next polling pass retries it.
pub async fn handle_openrouter_complete(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  api_key: &str,
  task: &Task,
  job: &VideoJob,
) {
  let result = handle_openrouter_complete_inner(
    app_handle,
    app_env_configs,
    app_data_root,
    task_database,
    storyteller_creds_manager,
    api_key,
    task,
    job,
  ).await;

  if let Err(err) = result {
    error!("[OpenrouterComplete] Task {} not saved yet, will retry: {}", task.id.as_str(), err);
  }
}

async fn handle_openrouter_complete_inner(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  api_key: &str,
  task: &Task,
  job: &VideoJob,
) -> Result<(), Box<dyn std::error::Error>> {
  // The upload needs a login. Wait for one rather than drop a video the user paid for.
  let creds = storyteller_creds_manager.get_credentials()?
    .ok_or("Not logged into ArtCraft")?;
  let content_url = job.unsigned_urls.first().ok_or("The job has no video URL")?;
  let host = &app_env_configs.storyteller_host;

  let bytes = download_video(api_key, content_url).await?;
  let download_path = app_data_root.temp_dir().path().join(format!("openrouter_{}.mp4", generate_random_uuid()));
  std::fs::write(&download_path, &bytes)?;

  let maybe_prompt_token = task.prompt_token.as_ref().map(|token| PromptToken::new_from_str(token));
  let upload_result = upload_video_media_file_from_file(UploadVideoFromFileArgs {
    api_host: host,
    maybe_creds: Some(&creds),
    path: &download_path,
    maybe_prompt_token: maybe_prompt_token.as_ref(),
    // Without a provider the server files this as a user upload, which My Library hides.
    // The server doesn't know OpenRouter, so name the app that generated it.
    maybe_generation_provider: Some(GenerationProvider::Artcraft),
  }).await;
  if let Err(err) = std::fs::remove_file(&download_path) {
    warn!("[OpenrouterComplete] Could not remove {:?}: {}", download_path, err);
  }
  let media_file_token = upload_result?.media_file_token;
  info!("[OpenrouterComplete] Task {} uploaded as {:?}", task.id.as_str(), media_file_token);

  // From here on, failing open: retrying would upload the video twice.
  let (maybe_cdn_url, maybe_thumbnail_url_template) = match get_media_file(host, &media_file_token).await {
    Ok(response) => {
      let links = &response.media_file.media_links;
      (Some(links.cdn_url.clone()), media_links_to_thumbnail_template(links).map(|template| template.to_string()))
    },
    Err(err) => {
      error!("[OpenrouterComplete] Failed to look up media file after upload: {:?} (failing open)", err);
      (None, None)
    },
  };

  if let Some(cdn_url) = maybe_cdn_url.as_ref() {
    if let Err(err) = auto_download_task_urls(app_handle, task, &[cdn_url.clone()]).await {
      warn!("[OpenrouterComplete] Automatic download failed for task {}: {:?}", task.id.as_str(), err);
    }
  }

  let maybe_cdn_url_str = maybe_cdn_url.as_ref().map(|url| url.to_string());
  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
    db: task_database.get_connection(),
    task_id: &task.id,
    maybe_batch_token: None,
    maybe_primary_media_file_token: Some(&media_file_token),
    maybe_primary_media_file_class: Some(TaskMediaFileClass::Video),
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
      media_class: TaskMediaFileClass::Video,
    };
    notify_frontend_of_completion(app_handle, host, Some(&creds), task, &completion).await;
  }

  info!("[OpenrouterComplete] Task {} fully handled", task.id.as_str());
  Ok(())
}
