import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../common/CommandStatus";

// Saved through the generic provider credential commands.
const OPENROUTER_API_KEY = "openrouter_api_key";

interface ProviderListSuccess extends CommandResult {
  payload: {
    providers: {
      provider_credential: string;
      has_credentials: boolean;
      maybe_details?: { maybe_key_start?: string | null } | null;
    }[];
  };
}

/** The saved key's first characters (`sk-or-********`), or undefined when none is saved. */
export const GetOpenrouterApiKeyHint = async (): Promise<string | undefined> => {
  const result = (await invoke("provider_list_command")) as ProviderListSuccess;
  const entry = result.payload.providers.find(
    (provider) => provider.provider_credential === OPENROUTER_API_KEY,
  );
  if (!entry?.has_credentials) return undefined;
  return entry.maybe_details?.maybe_key_start ?? "********";
};

export const SetOpenrouterApiKey = async (apiKey: string): Promise<void> => {
  await invoke("provider_set_api_key_command", {
    request: { provider_credential: OPENROUTER_API_KEY, api_key: apiKey.trim() },
  });
};

export const ClearOpenrouterApiKey = async (): Promise<void> => {
  await invoke("provider_clear_command", {
    request: { provider_credential: OPENROUTER_API_KEY },
  });
};
