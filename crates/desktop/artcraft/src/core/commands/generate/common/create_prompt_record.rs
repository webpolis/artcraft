use artcraft_api_defs::prompts::create_prompt::CreatePromptRequest;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::endpoints::prompts::create_prompt::create_prompt;
use artcraft_client::utils::api_host::ApiHost;
use log::warn;
use tokens::tokens::prompts::PromptToken;
use uuid_utils::uuid::generate_random_uuid;

/// Saves the prompt of a generation that runs outside ArtCraft's servers, so the library shows it
/// beside the result. Fails open.
pub async fn create_prompt_record(
  host: &ApiHost,
  creds: &StorytellerCredentialSet,
  prompt: &str,
  maybe_generate_audio: Option<bool>,
  maybe_duration_seconds: Option<u32>,
) -> Option<PromptToken> {
  let request = CreatePromptRequest {
    uuid_idempotency_token: generate_random_uuid(),
    positive_prompt: Some(prompt.to_string()),
    negative_prompt: None,
    model_type: None,
    // The ArtCraft server knows neither OpenRouter nor ComfyUI as a provider.
    generation_provider: None,
    maybe_generation_mode: None,
    maybe_aspect_ratio: None,
    maybe_resolution: None,
    maybe_batch_count: None,
    maybe_generate_audio,
    maybe_duration_seconds,
  };
  match create_prompt(host, Some(creds), request).await {
    Ok(response) => Some(response.prompt_token),
    Err(err) => {
      warn!("Could not create a prompt record (continuing): {:?}", err);
      None
    },
  }
}
