use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use log::error;
use openrouter_client::videos::download_video::download_video;
use openrouter_client::videos::video_job::VideoJob;
use sqlite_tasks::queries::task::Task;
use tauri::AppHandle;

use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::common::save_generated_file::{
  save_generated_file, GeneratedFile,
};
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

/// Downloads the finished video and saves it to the ArtCraft library. Any failure before the
/// upload leaves the task pending, so the next polling pass retries it.
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
  let bytes = download_video(api_key, content_url).await?;
  let file = GeneratedFile { bytes, extension: "mp4", media_class: TaskMediaFileClass::Video };
  save_generated_file(app_handle, app_env_configs, app_data_root, task_database, &creds, task, file).await
}
