import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../../../common/CommandStatus";
import { OmniGenVideoModelDetails } from "./ListVideoModels";

export interface ListOpenrouterVideoModelsSuccess extends CommandResult {
  payload: {
    models: OmniGenVideoModelDetails[];
  };
}

/**
 * OpenRouter's video models, already in the shape of the ArtCraft listing
 * (`list_openrouter_video_models_command`).
 */
export const ListOpenrouterVideoModels =
  async (): Promise<ListOpenrouterVideoModelsSuccess> =>
    (await invoke(
      "list_openrouter_video_models_command",
    )) as ListOpenrouterVideoModelsSuccess;
