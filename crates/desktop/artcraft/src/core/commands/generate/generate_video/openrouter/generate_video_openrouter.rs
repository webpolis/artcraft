//! Video generation with the user's own OpenRouter account. OpenRouter model ids
//! (`google/veo-3.1`) aren't ArtCraft models, so this reads the prompt box's fields directly
//! instead of decoding the native request. The third-party polling thread finishes the job.

use artcraft_client::utils::api_host::ApiHost;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info};
use openrouter_client::error::OpenrouterError;
use openrouter_client::videos::create_video::{create_video, CreateVideoRequest, FrameImage, FrameType, ImageInput};
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};
use tokens::tokens::media_files::MediaFileToken;

use crate::core::commands::enqueue::common::notify_frontend_of_errors::notify_frontend_of_errors;
use crate::core::commands::enqueue::generate_error::{GenerateError, MissingCredentialsReason};
use crate::core::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::core::commands::generate::common::create_prompt_record::create_prompt_record;
use crate::core::commands::generate::generate_image::providers::artcraft_router::utils::map_media_files_to_urls::map_media_file_tokens_to_cdn_urls;
use crate::core::commands::generate::generate_video::openrouter::openrouter_api_key::load_openrouter_api_key;
use crate::core::commands::generate::generate_video::openrouter::openrouter_values::{openrouter_aspect_ratio, openrouter_resolution};
use crate::core::commands::generate::omni::dispatch::{command_error, local_error};
use crate::core::commands::generate::omni::{OmniRequest, OmniResponse, OmniResult};
use crate::core::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::functional_events::show_provider_login_modal_event::ShowProviderLoginModalEvent;
use crate::core::events::generation_events::common::GenerationModel;
use crate::core::events::generation_events::generation_enqueue_success_event::GenerationEnqueueSuccessEvent;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;
use crate::core::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::core::state::task_database::TaskDatabase;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

type CommandError = CommandErrorResponseWrapper<String, ()>;

pub async fn generate_video_openrouter(request: OmniRequest, app: &AppHandle) -> OmniResult {
  let Some(api_key) = load_openrouter_api_key(&app.state::<ProviderCredentialLoadingCache>()) else {
    ShowProviderLoginModalEvent::send_for_provider(GenerationProvider::Openrouter, app);
    return Err(command_error(CommandErrorStatus::Unauthorized, "needs_openrouter_api_key", "Add your OpenRouter API key to generate with OpenRouter.".into()));
  };
  // Finished videos are uploaded to the ArtCraft library, which needs a login.
  let Some(creds) = app.state::<StorytellerCredentialManager>().get_credentials().map_err(local_error)? else {
    notify_frontend_of_errors(app, &GenerateError::MissingCredentials(MissingCredentialsReason::NeedsStorytellerCredentials)).await;
    return Err(command_error(CommandErrorStatus::Unauthorized, "needs_storyteller_credentials", "You need to be logged into ArtCraft.".into()));
  };
  let host = &app.state::<AppEnvConfigs>().storyteller_host;

  let body = build_request(&request.fields, host).await?;
  let maybe_prompt_token = create_prompt_record(host, &creds, &body.prompt, body.generate_audio, body.duration).await;
  let job = create_video(&api_key, &body).await.map_err(openrouter_command_error)?;
  info!("OpenRouter accepted video job {} for {}", job.id, body.model);

  let success = TaskEnqueueSuccess {
    task_type: TaskType::VideoGeneration,
    model: Some(GenerationModel::Unknown(body.model)),
    provider: GenerationProvider::Openrouter,
    provider_job_id: Some(job.id),
    maybe_queue_status_url: Some(job.polling_url),
    maybe_queue_response_url: None,
    maybe_prompt_token,
  };
  if let Err(err) = success.insert_into_task_database_with_frontend_payload(&app.state::<TaskDatabase>(), request.frontend_caller, request.frontend_subscriber_id.as_deref(), request.frontend_subscriber_payload.as_deref()).await {
    // OpenRouter is already generating (and billing). Do not encourage a duplicate retry.
    error!("Could not persist accepted OpenRouter job: {:?}", err);
  }
  GenerationEnqueueSuccessEvent { action: success.to_frontend_event_action(), service: success.to_frontend_event_service(), model: success.model }.send_infallible(app);
  Ok(OmniResponse(Value::Object(Map::new())).into())
}

async fn build_request(fields: &Map<String, Value>, host: &ApiHost) -> Result<CreateVideoRequest, CommandError> {
  let model = string_field(fields, "model")
    .ok_or_else(|| command_error(CommandErrorStatus::BadRequest, "model_not_specified", "No model specified for video generation".into()))?;

  let mut frame_images = Vec::new();
  for (field, frame_type) in [("start_frame_image_media_token", FrameType::FirstFrame), ("end_frame_image_media_token", FrameType::LastFrame)] {
    if let Some(token) = string_field(fields, field) {
      for url in resolve_urls(&[token], host).await? {
        frame_images.push(FrameImage { image: ImageInput::from_url(url), frame_type });
      }
    }
  }

  let reference_tokens: Vec<String> = fields.get("reference_image_media_tokens")
    .and_then(Value::as_array)
    .map(|tokens| tokens.iter().filter_map(Value::as_str).map(str::to_owned).collect())
    .unwrap_or_default();
  let input_references = resolve_urls(&reference_tokens, host).await?
    .into_iter()
    .map(ImageInput::from_url)
    .collect();

  Ok(CreateVideoRequest {
    model,
    prompt: string_field(fields, "prompt").unwrap_or_default(),
    duration: fields.get("duration_seconds").and_then(Value::as_u64).map(|seconds| seconds as u32),
    resolution: string_field(fields, "resolution").map(|value| openrouter_resolution(&value)),
    aspect_ratio: string_field(fields, "aspect_ratio").map(|value| openrouter_aspect_ratio(&value)),
    generate_audio: fields.get("generate_audio").and_then(Value::as_bool),
    frame_images,
    input_references,
  })
}

/// Media tokens point at public ArtCraft CDN files, which OpenRouter's providers can fetch.
async fn resolve_urls(tokens: &[String], host: &ApiHost) -> Result<Vec<String>, CommandError> {
  let tokens: Vec<MediaFileToken> = tokens.iter().map(|token| MediaFileToken::new_from_str(token)).collect();
  map_media_file_tokens_to_cdn_urls(&tokens, host).await
    .map_err(|err| local_error(format!("Could not resolve reference images: {:?}", err)))
}

fn openrouter_command_error(err: OpenrouterError) -> CommandError {
  error!("OpenRouter video submit failed: {:?}", err);
  match err {
    OpenrouterError::Api { status: 401 | 403, message } =>
      command_error(CommandErrorStatus::Unauthorized, "openrouter_unauthorized", format!("OpenRouter rejected your API key: {}", message)),
    OpenrouterError::Api { status: 429, message } =>
      command_error(CommandErrorStatus::TooManyRequests, "too_many_requests", format!("OpenRouter: {}", message)),
    OpenrouterError::Api { status: 400..=499, message } =>
      command_error(CommandErrorStatus::BadRequest, "bad_input", format!("OpenRouter: {}", message)),
    other => local_error(format!("OpenRouter request failed: {}", other)),
  }
}

fn string_field(fields: &Map<String, Value>, name: &str) -> Option<String> {
  fields.get(name)
    .and_then(Value::as_str)
    .filter(|value| !value.is_empty())
    .map(str::to_owned)
}
