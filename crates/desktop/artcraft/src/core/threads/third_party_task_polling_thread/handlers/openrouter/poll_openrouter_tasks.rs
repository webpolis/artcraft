use log::{error, info, warn};
use openrouter_client::error::OpenrouterError;
use openrouter_client::videos::get_video::get_video;
use openrouter_client::videos::video_job::VideoJobStatus;
use sqlite_tasks::queries::task::Task;
use tauri::AppHandle;

use crate::core::commands::generate::generate_video::openrouter::openrouter_api_key::load_openrouter_api_key;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::common::mark_task_failed::mark_task_failed;
use crate::core::threads::third_party_task_polling_thread::handlers::openrouter::handle_openrouter_complete::handle_openrouter_complete;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

pub async fn poll_openrouter_tasks(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  credential_cache: &ProviderCredentialLoadingCache,
  tasks: &[&Task],
) {
  let Some(api_key) = load_openrouter_api_key(credential_cache) else {
    warn!("[OpenrouterPolling] No OpenRouter API key configured, skipping poll");
    return;
  };

  for task in tasks {
    let result = poll_single_task(
      app_handle,
      app_env_configs,
      app_data_root,
      task_database,
      storyteller_creds_manager,
      &api_key,
      task,
    ).await;

    if let Err(err) = result {
      error!("[OpenrouterPolling] Error checking task {}: {:?}", task.id.as_str(), err);
    }
  }
}

async fn poll_single_task(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  api_key: &str,
  task: &Task,
) -> Result<(), OpenrouterError> {
  let Some(polling_url) = task.queue_status_url.as_deref() else {
    mark_task_failed(app_handle, task_database, task, "The OpenRouter job has no polling URL").await;
    return Ok(());
  };

  let job = match get_video(api_key, polling_url).await {
    Ok(job) => job,
    Err(OpenrouterError::Api { status: 404, .. }) => {
      mark_task_failed(app_handle, task_database, task, "OpenRouter no longer has this job").await;
      return Ok(());
    },
    Err(err) => return Err(err),
  };

  match job.status {
    VideoJobStatus::Completed if job.unsigned_urls.is_empty() => {
      mark_task_failed(app_handle, task_database, task, "OpenRouter finished the job but returned no video").await;
    },
    VideoJobStatus::Completed => {
      info!("[OpenrouterPolling] Task {} completed, saving the video...", task.id.as_str());
      handle_openrouter_complete(
        app_handle,
        app_env_configs,
        app_data_root,
        task_database,
        storyteller_creds_manager,
        api_key,
        task,
        &job,
      ).await;
    },
    VideoJobStatus::Failed | VideoJobStatus::Cancelled | VideoJobStatus::Expired => {
      let reason = job.error.unwrap_or_else(|| format!("The OpenRouter job ended as {:?}", job.status));
      mark_task_failed(app_handle, task_database, task, &reason).await;
    },
    VideoJobStatus::Pending | VideoJobStatus::InProgress | VideoJobStatus::Unknown => {
      info!("[OpenrouterPolling] Task {} still {:?}", task.id.as_str(), job.status);
    },
  }

  Ok(())
}
