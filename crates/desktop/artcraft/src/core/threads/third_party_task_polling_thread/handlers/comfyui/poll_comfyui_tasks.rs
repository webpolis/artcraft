use comfyui_client::error::ComfyuiError;
use comfyui_client::files::download_file;
use comfyui_client::prompts::{get_prompt_state, PromptState};
use enums::tauri::tasks::task_media_file_class::TaskMediaFileClass;
use enums::tauri::tasks::task_type::TaskType;
use log::{info, warn};
use sqlite_tasks::queries::task::Task;
use tauri::AppHandle;

use crate::core::commands::generate::comfyui::COMFYUI_HOST;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::handlers::common::mark_task_failed::mark_task_failed;
use crate::core::threads::third_party_task_polling_thread::handlers::common::save_generated_file::{
  save_generated_file, GeneratedFile,
};
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

pub async fn poll_comfyui_tasks(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  tasks: &[&Task],
) {
  for task in tasks {
    let result = poll_single_task(
      app_handle,
      app_env_configs,
      app_data_root,
      task_database,
      storyteller_creds_manager,
      task,
    ).await;

    if let Err(err) = result {
      warn!("[ComfyuiPolling] Task {} not finished yet: {}", task.id.as_str(), err);
    }
  }
}

async fn poll_single_task(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  task: &Task,
) -> Result<(), Box<dyn std::error::Error>> {
  let Some(prompt_id) = task.provider_job_id.as_deref() else {
    mark_task_failed(app_handle, task_database, task, "The ComfyUI job has no prompt id").await;
    return Ok(());
  };

  let state = match get_prompt_state(COMFYUI_HOST, prompt_id).await {
    // ArtCraft starts ComfyUI and stops it on exit, so a silent server lost the job.
    Err(ComfyuiError::Http(_)) => {
      mark_task_failed(app_handle, task_database, task, "ComfyUI stopped before this job finished").await;
      return Ok(());
    },
    other => other?,
  };
  let files = match state {
    PromptState::Queued => return Ok(()),
    PromptState::Failed(reason) => {
      mark_task_failed(app_handle, task_database, task, &reason).await;
      return Ok(());
    },
    PromptState::Missing => {
      mark_task_failed(app_handle, task_database, task, "ComfyUI no longer has this job. Was it restarted?").await;
      return Ok(());
    },
    PromptState::Finished(files) => files,
  };

  let Some(file) = files.first() else {
    mark_task_failed(app_handle, task_database, task, "The ComfyUI workflow saved no file").await;
    return Ok(());
  };
  info!("[ComfyuiPolling] Task {} finished as {}, saving it...", task.id.as_str(), file.filename);

  // The upload needs a login. Wait for one rather than drop the result.
  let creds = storyteller_creds_manager.get_credentials()?
    .ok_or("Not logged into ArtCraft")?;
  let bytes = download_file(COMFYUI_HOST, file).await?;
  let (extension, media_class) = match task.task_type {
    TaskType::VideoGeneration => ("mp4", TaskMediaFileClass::Video),
    _ => ("png", TaskMediaFileClass::Image),
  };
  let generated = GeneratedFile { bytes, extension, media_class };
  save_generated_file(app_handle, app_env_configs, app_data_root, task_database, &creds, task, generated).await
}
