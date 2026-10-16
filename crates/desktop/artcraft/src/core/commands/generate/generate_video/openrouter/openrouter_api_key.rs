use crate::core::providers::credentials::payload::provider_credential_payload::ProviderCredentialPayload;
use crate::core::providers::credentials::provider_credential_key::ProviderCredentialKey;
use crate::core::providers::credentials::provider_credential_loading_cache::ProviderCredentialLoadingCache;

/// The key saved in Settings (`~/Artcraft/credentials/openrouter.api_key.txt`), if any.
pub fn load_openrouter_api_key(credential_cache: &ProviderCredentialLoadingCache) -> Option<String> {
  match credential_cache.get_credentials(ProviderCredentialKey::OpenrouterApiKey) {
    Ok(Some(ProviderCredentialPayload::ApiKey(data))) => Some(data.as_str().to_string()),
    _ => None,
  }
}
