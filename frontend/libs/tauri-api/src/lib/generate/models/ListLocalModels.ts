import { invoke } from "@tauri-apps/api/core";
import { CommandResult } from "../../common/CommandStatus";
import { OmniGenImageModelDetails } from "./image/ListImageModels";
import { OmniGenVideoModelDetails } from "./video/ListVideoModels";

export interface ListLocalModelsSuccess extends CommandResult {
  payload: {
    video_models: OmniGenVideoModelDetails[];
    image_models: OmniGenImageModelDetails[];
  };
}

/**
 * Models that run on the local ComfyUI server, already in the shape of the
 * ArtCraft listing (`list_local_models_command`).
 */
export const ListLocalModels = async (): Promise<ListLocalModelsSuccess> =>
  (await invoke("list_local_models_command")) as ListLocalModelsSuccess;
