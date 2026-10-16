//! Generation on the local ComfyUI server. The model id picks the workflow, and so whether this is
//! a video or an image; the third-party polling thread saves the result to the library.

use artcraft_client::utils::api_host::ApiHost;
use comfyui_client::error::ComfyuiError;
use comfyui_client::files::upload_image;
use comfyui_client::prompts::queue_prompt;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info};
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};
use tokens::tokens::media_files::MediaFileToken;

use crate::core::commands::enqueue::common::notify_frontend_of_errors::notify_frontend_of_errors;
use crate::core::commands::enqueue::generate_error::{GenerateError, MissingCredentialsReason};
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::comfyui::comfyui_launcher::ensure_comfyui_running;
use crate::core::commands::generate::comfyui::minimax_h3_workflow::{self, MinimaxH3Input};
use crate::core::commands::generate::comfyui::qwen_image_workflow::{self, QwenImageInput};
use crate::core::commands::generate::comfyui::{COMFYUI_HOST, MINIMAX_H3_MODEL, QWEN_IMAGE_MODEL};
use crate::core::commands::generate::common::create_prompt_record::create_prompt_record;
use crate::core::commands::generate::generate_image::providers::artcraft_router::utils::map_media_files_to_urls::map_media_file_tokens_to_cdn_urls;
use crate::core::commands::generate::omni::dispatch::{command_error, local_error};
use crate::core::commands::generate::omni::{OmniRequest, OmniResponse, OmniResult};
use crate::core::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::events::generation_events::generation_enqueue_success_event::GenerationEnqueueSuccessEvent;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use crate::core::state::task_database::TaskDatabase;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

type CommandError = CommandErrorResponseWrapper<String, ()>;

/// What the prompt box asked for, as a workflow and the facts the task record needs.
struct LocalJob {
  workflow: Value,
  task_type: TaskType,
  maybe_generate_audio: Option<bool>,
  maybe_duration_seconds: Option<u32>,
}

pub async fn generate_with_comfyui(request: OmniRequest, app: &AppHandle) -> OmniResult {
  // Finished results are uploaded to the ArtCraft library, which needs a login.
  let Some(creds) = app.state::<StorytellerCredentialManager>().get_credentials().map_err(local_error)? else {
    notify_frontend_of_errors(app, &GenerateError::MissingCredentials(MissingCredentialsReason::NeedsStorytellerCredentials)).await;
    return Err(command_error(CommandErrorStatus::Unauthorized, "needs_storyteller_credentials", "You need to be logged into ArtCraft.".into()));
  };
  let configs = app.state::<AppEnvConfigs>();
  let log_path = app.state::<AppDataRoot>().temp_dir().path().join("comfyui.log");
  ensure_comfyui_running(configs.maybe_comfyui_launcher.as_deref(), &log_path).await
    .map_err(|message| command_error(CommandErrorStatus::ServerError, "comfyui_unreachable", message))?;
  let host = &configs.storyteller_host;
  let fields = &request.fields;
  let model = string_field(fields, "model")
    .ok_or_else(|| command_error(CommandErrorStatus::BadRequest, "model_not_specified", "No model specified".into()))?;
  let prompt = string_field(fields, "prompt").unwrap_or_default();

  let job = build_job(&model, &prompt, fields, host).await?;
  let maybe_prompt_token = create_prompt_record(host, &creds, &prompt, job.maybe_generate_audio, job.maybe_duration_seconds).await;
  let prompt_id = queue_prompt(COMFYUI_HOST, &job.workflow).await.map_err(comfyui_command_error)?;
  info!("ComfyUI accepted {} as prompt {}", model, prompt_id);

  let success = TaskEnqueueSuccess {
    task_type: job.task_type,
    model: Some(GenerationModel::Unknown(model)),
    provider: GenerationProvider::Comfyui,
    provider_job_id: Some(prompt_id),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token,
  };
  if let Err(err) = success.insert_into_task_database_with_frontend_payload(&app.state::<TaskDatabase>(), request.frontend_caller, request.frontend_subscriber_id.as_deref(), request.frontend_subscriber_payload.as_deref()).await {
    error!("Could not persist accepted ComfyUI job: {:?}", err);
  }
  GenerationEnqueueSuccessEvent { action: success.to_frontend_event_action(), service: success.to_frontend_event_service(), model: success.model }.send_infallible(app);
  Ok(OmniResponse(Value::Object(Map::new())).into())
}

async fn build_job(model: &str, prompt: &str, fields: &Map<String, Value>, host: &ApiHost) -> Result<LocalJob, CommandError> {
  let seed = u64::from(rand::random::<u32>());
  match model {
    MINIMAX_H3_MODEL => {
      let first_frame = upload_frame(fields, "start_frame_image_media_token", host).await?;
      let last_frame = upload_frame(fields, "end_frame_image_media_token", host).await?;
      let generate_audio = fields.get("generate_audio").and_then(Value::as_bool).unwrap_or(true);
      let duration_seconds = fields.get("duration_seconds").and_then(Value::as_u64).map(|seconds| seconds as u32);
      let workflow = minimax_h3_workflow::build_workflow(&MinimaxH3Input {
        prompt,
        aspect_ratio: fields.get("aspect_ratio").and_then(Value::as_str),
        resolution: fields.get("resolution").and_then(Value::as_str),
        duration_seconds,
        generate_audio,
        seed,
        first_frame: first_frame.as_deref(),
        last_frame: last_frame.as_deref(),
      });
      Ok(LocalJob { workflow, task_type: TaskType::VideoGeneration, maybe_generate_audio: Some(generate_audio), maybe_duration_seconds: duration_seconds })
    },
    QWEN_IMAGE_MODEL => {
      let workflow = qwen_image_workflow::build_workflow(&QwenImageInput {
        prompt,
        aspect_ratio: fields.get("aspect_ratio").and_then(Value::as_str),
        seed,
      });
      Ok(LocalJob { workflow, task_type: TaskType::ImageGeneration, maybe_generate_audio: None, maybe_duration_seconds: None })
    },
    other => Err(command_error(CommandErrorStatus::BadRequest, "bad_input", format!("Unknown local model {}", other))),
  }
}

/// Copies an ArtCraft media file into ComfyUI's input folder and returns its name there.
async fn upload_frame(fields: &Map<String, Value>, field: &str, host: &ApiHost) -> Result<Option<String>, CommandError> {
  let Some(token) = string_field(fields, field) else {
    return Ok(None);
  };
  let urls = map_media_file_tokens_to_cdn_urls(&[MediaFileToken::new_from_str(&token)], host).await
    .map_err(|err| local_error(format!("Could not resolve the frame image: {:?}", err)))?;
  let url = urls.first().ok_or_else(|| local_error("The frame image has no URL"))?;
  let bytes = reqwest::get(url).await
    .and_then(|response| response.error_for_status())
    .map_err(|err| local_error(format!("Could not download the frame image: {}", err)))?
    .bytes().await
    .map_err(|err| local_error(format!("Could not download the frame image: {}", err)))?;
  let name = upload_image(COMFYUI_HOST, bytes.into(), &format!("artcraft_{}.png", token)).await
    .map_err(comfyui_command_error)?;
  Ok(Some(name))
}

fn comfyui_command_error(err: ComfyuiError) -> CommandError {
  error!("ComfyUI request failed: {:?}", err);
  match err {
    ComfyuiError::Http(_) => command_error(CommandErrorStatus::ServerError, "comfyui_unreachable",
      format!("ComfyUI stopped answering at {}.", COMFYUI_HOST)),
    ComfyuiError::Api { message, .. } => command_error(CommandErrorStatus::BadRequest, "bad_input",
      format!("ComfyUI rejected the workflow: {}", message)),
    other => local_error(format!("ComfyUI request failed: {}", other)),
  }
}

fn string_field(fields: &Map<String, Value>, name: &str) -> Option<String> {
  fields.get(name)
    .and_then(Value::as_str)
    .filter(|value| !value.is_empty())
    .map(str::to_owned)
}
