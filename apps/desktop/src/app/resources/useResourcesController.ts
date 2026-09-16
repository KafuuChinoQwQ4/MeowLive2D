import { useCallback, useEffect, useRef, useState } from "react";
import type {
  CharacterPreviewRequest,
  CharacterSaveRequest,
  DesktopResourceResult,
  ResourceSnapshot,
  SpeechSnapshot,
  VoiceCreateRequest,
  VtsHotkey,
  VtsModel,
} from "@meowlive/contracts";
import type { ServerClient } from "../../services/server";
import type { ResourcesClient } from "../../services/server/resources";

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : "资源操作失败，请稍后重试。";
}

export function useResourcesController(client: ResourcesClient, speechClient: ServerClient) {
  const [snapshot, setSnapshot] = useState<ResourceSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [models, setModels] = useState<VtsModel[]>([]);
  const [hotkeys, setHotkeys] = useState<VtsHotkey[]>([]);
  const [hotkeyModelId, setHotkeyModelId] = useState<string | null>(null);
  const [voicePreview, setVoicePreview] = useState<SpeechSnapshot | null>(null);
  const [importResult, setImportResult] = useState<Extract<DesktopResourceResult, { type: "model_imported" }> | null>(null);
  const activeAbort = useRef<AbortController | null>(null);
  const loadGeneration = useRef(0);
  const pendingRef = useRef(false);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    const generation = ++loadGeneration.current;
    const controller = new AbortController();
    activeAbort.current = controller;
    void client.getSnapshot(controller.signal).then((value) => {
      if (!mounted.current || controller.signal.aborted || loadGeneration.current !== generation) return;
      setSnapshot(value);
      setLoadError(null);
    }).catch((error: unknown) => {
      if (!mounted.current || controller.signal.aborted || loadGeneration.current !== generation) return;
      setLoadError(messageOf(error));
    }).finally(() => {
      if (mounted.current && loadGeneration.current === generation) setLoading(false);
      if (activeAbort.current === controller) activeAbort.current = null;
    });
    return () => {
      mounted.current = false;
      loadGeneration.current += 1;
      controller.abort();
      if (activeAbort.current === controller) activeAbort.current = null;
      else activeAbort.current?.abort();
    };
  }, [client]);

  const run = useCallback(async <T,>(kind: string, operation: (signal: AbortSignal) => Promise<T>): Promise<T | null> => {
    if (pendingRef.current) return null;
    pendingRef.current = true;
    setPendingAction(kind);
    setActionError(null);
    const controller = new AbortController();
    activeAbort.current = controller;
    try {
      return await operation(controller.signal);
    } catch (error) {
      if (mounted.current && !controller.signal.aborted) setActionError(messageOf(error));
      return null;
    } finally {
      if (mounted.current) setPendingAction(null);
      pendingRef.current = false;
      if (activeAbort.current === controller) activeAbort.current = null;
    }
  }, []);

  const updateSnapshot = useCallback(async (
    kind: string,
    operation: (signal: AbortSignal) => Promise<ResourceSnapshot>,
  ) => {
    const value = await run(kind, operation);
    if (value && mounted.current) setSnapshot(value);
    return value;
  }, [run]);

  return {
    snapshot,
    loading,
    loadError,
    actionError,
    pendingAction,
    models,
    hotkeys,
    hotkeyModelId,
    voicePreview,
    importResult,
    uploadVoice: (metadata: VoiceCreateRequest, audio: File) =>
      updateSnapshot("voice-upload", (signal) => client.createVoice(metadata, audio, signal)),
    selectVoice: (id: string) =>
      updateSnapshot("voice-select", (signal) => client.selectVoice({ id }, signal)),
    previewVoice: async (voiceId: string, text: string) => {
      const value = await run("voice-preview", (signal) =>
        speechClient.submitSpeech({ text, voice_id: voiceId }, signal));
      if (value && mounted.current) setVoicePreview(value);
      return value;
    },
    saveCharacter: (request: CharacterSaveRequest) =>
      updateSnapshot("character-save", (signal) => client.saveCharacter(request, signal)),
    selectCharacter: (id: string) =>
      updateSnapshot("character-select", (signal) => client.selectCharacter({ id }, signal)),
    previewCharacter: (request: CharacterPreviewRequest) =>
      updateSnapshot("character-preview", (signal) => client.previewCharacter(request, signal)),
    refreshModels: async () => {
      const value = await run("models", (signal) => client.desktop({ type: "list_models" }, signal));
      if (value?.type === "models" && mounted.current) setModels(value.models);
      if (value?.type === "error" && mounted.current) setActionError(value.message);
      return value;
    },
    refreshHotkeys: async (modelId: string) => {
      const value = await run("hotkeys", (signal) =>
        client.desktop({ type: "list_hotkeys", model_id: modelId }, signal));
      if (value?.type === "hotkeys" && mounted.current) {
        setHotkeys(value.hotkeys);
        setHotkeyModelId(value.model_id);
      }
      if (value?.type === "error" && mounted.current) setActionError(value.message);
      return value;
    },
    importModel: async () => {
      const imported = await run("model-import", (signal) =>
        client.desktop({ type: "import_model" }, signal));
      if (!imported || !mounted.current) return null;
      if (imported.type === "error") {
        setActionError(imported.message);
        return imported;
      }
      if (imported.type !== "model_imported") return imported;
      setImportResult(imported);

      const refreshed = await run("models", (signal) =>
        client.desktop({ type: "list_models" }, signal));
      if (refreshed?.type === "models" && mounted.current) setModels(refreshed.models);
      if (refreshed?.type === "error" && mounted.current) setActionError(refreshed.message);
      return imported;
    },
  };
}

export type ResourcesController = ReturnType<typeof useResourcesController>;
