import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";
import { useEffect, useState } from "react";
import { LoaderCircleIcon } from "lucide-react";
import {
  ClearOpenrouterApiKey,
  GetOpenrouterApiKeyHint,
  SetOpenrouterApiKey,
} from "@storyteller/tauri-api";

export const OpenrouterAccountBlock = () => {
  const [keyHint, setKeyHint] = useState<string | undefined>(undefined);
  const [draftKey, setDraftKey] = useState("");
  const [isBusy, setIsBusy] = useState(false);

  const refreshKeyHint = async () => {
    setIsBusy(true);
    try {
      setKeyHint(await GetOpenrouterApiKeyHint());
    } catch (e) {
      console.error("Error reading OpenRouter API key", e);
      setKeyHint(undefined);
    } finally {
      setIsBusy(false);
    }
  };

  useEffect(() => {
    refreshKeyHint();
  }, []);

  const handleButton = async () => {
    try {
      if (keyHint) {
        await ClearOpenrouterApiKey();
      } else {
        await SetOpenrouterApiKey(draftKey);
        setDraftKey("");
      }
    } catch (e) {
      console.error("Error saving OpenRouter API key", e);
    }
    await refreshKeyHint();
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-0.5">
          <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.12em]">
            OpenRouter API key
          </p>
          <p className="truncate text-sm font-medium text-base-fg/80">
            {keyHint ?? "Not set"}
          </p>
        </div>
        <Button
          variant={keyHint ? "destructive" : "secondary"}
          className="h-9 shrink-0 px-3"
          onClick={handleButton}
          disabled={isBusy || (!keyHint && !draftKey.trim())}
        >
          {isBusy ? (
            <LoaderCircleIcon className="animate-spin text-sm" />
          ) : keyHint ? (
            "Remove"
          ) : (
            "Save"
          )}
        </Button>
      </div>
      {!keyHint && (
        <Input
          type="password"
          value={draftKey}
          onChange={(e) => setDraftKey(e.target.value)}
          placeholder="Paste your OpenRouter API key"
        />
      )}
    </div>
  );
};
