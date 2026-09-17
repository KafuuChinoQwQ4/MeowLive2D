import { useCallback, useEffect, useRef, useState } from "react";
import type {
  CharacterPreviewRequest,
  CharacterSaveRequest,
  DesktopResourceResult,
  ImportedModel,
  ResourceSnapshot,
  SpeechSnapshot,
  VoiceCreateRequest,
  VtsHotkey,
  VtsModel,
} from "@meowlive/contracts";
import type { ServerClient } from "../../services/server";
import type { ResourcesClient } from "../../services/server/resources";
import { useFeedback } from "../feedback/OperationFeedback";

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : "资源操作失败，请稍后重试。";
}

const activePreviewStatuses = new Set(["queued", "synthesizing", "ready", "playing"]);
const operationLabels: Record<string, string> = {
  "voice-upload": "上传音色", "voice-select": "选择音色", "voice-delete": "删除音色", "voice-preview": "试听音色",
  "character-save": "保存角色", "character-select": "加载角色", "character-delete": "删除角色", "character-preview": "预览角色热键",
  models: "刷新 VTS 模型", hotkeys: "刷新角色热键", "installed-models": "刷新已安装模型", "model-import": "导入模型", "model-delete": "删除模型",
};

export function useResourcesController(client: ResourcesClient, speechClient: ServerClient) {
  const feedback = useFeedback();
  const [snapshot, setSnapshot] = useState<ResourceSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [models, setModels] = useState<VtsModel[]>([]);
  const [installedModels, setInstalledModels] = useState<ImportedModel[] | null>(null);
  const [modelNotice, setModelNotice] = useState<string | null>(null);
  const [voiceDeleteRetries, setVoiceDeleteRetries] = useState<string[]>([]);
  const [hotkeys, setHotkeys] = useState<VtsHotkey[]>([]);
  const [hotkeyModelId, setHotkeyModelId] = useState<string | null>(null);
  const [voicePreview, setVoicePreview] = useState<SpeechSnapshot | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [importResult, setImportResult] = useState<Extract<DesktopResourceResult, { type: "model_imported" }> | null>(null);
  const activeAbort = useRef<AbortController | null>(null);
  const loadGeneration = useRef(0);
  const pendingRef = useRef(false);
  const mounted = useRef(true);
  const previewAbort = useRef<AbortController | null>(null);
  const previewId = voicePreview?.id;
  const previewGeneration = voicePreview?.generation;
  const previewActive = voicePreview !== null && activePreviewStatuses.has(voicePreview.status);
  useEffect(() => {
    if (previewError) feedback.reportIssue("resources:preview", "试听状态异常", previewError);
    else feedback.clearIssue("resources:preview");
  }, [previewError, feedback]);

  useEffect(() => {
    if (!previewActive || previewId === undefined) return;
    const controller = new AbortController();
    previewAbort.current = controller;
    let timer: ReturnType<typeof setTimeout>;
    async function refreshPreview() {
      let keepPolling = true;
      try {
        const status = await speechClient.getStatus(controller.signal);
        if (controller.signal.aborted) return;
        const next = status.speeches.find(task => task.id === previewId && task.generation === previewGeneration);
        if (next) {
          setVoicePreview(next);
          keepPolling = activePreviewStatuses.has(next.status);
          if (next.status === "completed") feedback.success("音色试听完成", "参考音色已播放完成。");
          if (next.status === "failed" || next.status === "unknown") feedback.reportIssue(`resources:preview:${next.generation}:${next.id}`, "音色试听失败", next.error || "播放结果未知，请检查执行端连接。");
          setPreviewError(!status.bridge_connected && next.status !== "completed"
            ? "Windows 执行端已断开，请重新连接后再次试听。" : null);
        } else {
          setVoicePreview(current => current && { ...current, status: "unknown" });
          setPreviewError("试听记录已不可用，请检查主服务和 Windows 执行端后再次试听。");
          keepPolling = false;
        }
      } catch (error) {
        if (controller.signal.aborted) return;
        setPreviewError(`无法读取试听状态：${messageOf(error)}`);
      } finally {
        if (!controller.signal.aborted && keepPolling) timer = setTimeout(() => { void refreshPreview(); }, 1000);
      }
    }
    timer = setTimeout(() => { void refreshPreview(); }, 1000);
    return () => {
      controller.abort();
      clearTimeout(timer);
      if (previewAbort.current === controller) previewAbort.current = null;
    };
  }, [speechClient, previewId, previewGeneration, previewActive, feedback]);

  useEffect(() => {
    mounted.current = true;
    const generation = ++loadGeneration.current;
    const controller = new AbortController();
    activeAbort.current = controller;
    void client.getSnapshot(controller.signal).then((value) => {
      if (!mounted.current || controller.signal.aborted || loadGeneration.current !== generation) return;
      setSnapshot(value);
      setLoadError(null);
      feedback.clearIssue("resources:load");
    }).catch((error: unknown) => {
      if (!mounted.current || controller.signal.aborted || loadGeneration.current !== generation) return;
      setLoadError(messageOf(error));
      feedback.reportIssue("resources:load", "资源读取失败", error);
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
  }, [client, feedback]);

  const run = useCallback(async <T,>(kind: string, operation: (signal: AbortSignal) => Promise<T>, announce = true): Promise<T | null> => {
    if (pendingRef.current) return null;
    pendingRef.current = true;
    setPendingAction(kind);
    setActionError(null);
    const controller = new AbortController();
    activeAbort.current = controller;
    try {
      const result = await operation(controller.signal);
      if (!mounted.current || controller.signal.aborted) return null;
      if (result && typeof result === "object" && "type" in result && result.type === "error" && "message" in result) throw new Error(String(result.message));
      if (kind === "voice-preview") {
        const preview = result as SpeechSnapshot;
        if (preview.status === "failed" || preview.status === "unknown") throw new Error(preview.error || "试听结果未知，请检查执行端。");
      }
      if (announce) {
        const label = operationLabels[kind] ?? "资源操作";
        const message = kind === "voice-preview" ? "试听任务已接收，请查看试听状态确认播放结果。"
          : kind === "model-import" || kind === "model-delete" ? "模型文件已更新，请重启 VTube Studio 后刷新模型列表。"
          : `${label}已完成。`;
        feedback.notify({ kind: kind === "voice-preview" ? "info" : "success", title: kind === "voice-preview" ? "音色试听已提交" : `${label}成功`, message });
      }
      return result;
    } catch (error) {
      if (mounted.current && !controller.signal.aborted) { setActionError(messageOf(error)); feedback.error(`${operationLabels[kind] ?? "资源操作"}失败`, error); }
      return null;
    } finally {
      if (mounted.current) setPendingAction(null);
      pendingRef.current = false;
      if (activeAbort.current === controller) activeAbort.current = null;
    }
  }, [feedback]);

  const updateSnapshot = useCallback(async (
    kind: string,
    operation: (signal: AbortSignal) => Promise<ResourceSnapshot>,
  ) => {
    const value = await run(kind, operation);
    if (value && mounted.current) setSnapshot(value);
    return value;
  }, [run]);

  const deleteResource = async (kind: string, operation: (signal: AbortSignal) => Promise<ResourceSnapshot>) => {
    return updateSnapshot(kind, async signal => {
      try { return await operation(signal); }
      catch (error) {
        // Cleanup can fail after the metadata commit; refresh before reporting the error.
        try {
          const current = await client.getSnapshot(signal);
          if (mounted.current && !signal.aborted) setSnapshot(current);
        } catch { /* Keep the original deletion error. */ }
        throw error;
      }
    });
  };

  const refreshInstalledModels = async (announce = true) => {
    const value = await run("installed-models", signal => client.desktop({ type: "list_imported_models" }, signal), announce);
    if (value?.type === "imported_models" && mounted.current) setInstalledModels(value.models);
    if (value?.type === "error" && mounted.current) setActionError(value.message);
    return value;
  };

  return {
    snapshot,
    loading,
    loadError,
    actionError: [actionError, previewError].filter(Boolean).join("；") || null,
    pendingAction,
    models,
    installedModels,
    modelNotice,
    refreshInstalledModels,
    deleteModel: async (model: ImportedModel) => {
      const value = await run("model-delete", async signal => {
        const result = await client.desktop({ type: "delete_imported_model", id: model.id }, signal);
        if (result.type === "error") throw new Error(result.message);
        if (result.type === "model_deleted" && mounted.current) {
          setInstalledModels(current => current?.filter(item => item.id !== model.id) ?? null);
          setModels(current => current.filter(item => item.id !== model.model_id));
          if (hotkeyModelId === model.model_id) { setHotkeys([]); setHotkeyModelId(null); }
          setImportResult(null);
          setModelNotice(`已删除模型“${model.name}”，请重启 VTube Studio 后刷新模型列表。`);
        }
        return result;
      });
      return value;
    },
    hotkeys,
    hotkeyModelId,
    voicePreview,
    voiceDeleteRetries,
    importResult,
    uploadVoice: (metadata: VoiceCreateRequest, audio: File) =>
      updateSnapshot("voice-upload", (signal) => client.createVoice(metadata, audio, signal)),
    selectVoice: (id: string) =>
      updateSnapshot("voice-select", (signal) => client.selectVoice({ id }, signal)),
    deleteVoice: async (id: string) => {
      const value = await deleteResource("voice-delete", async signal => {
        try { return await client.deleteVoice({ id }, signal); }
        catch (error) {
          if (mounted.current && messageOf(error).includes("清理失败")) setVoiceDeleteRetries(current => current.includes(id) ? current : [...current, id]);
          throw error;
        }
      });
      if (value && mounted.current) setVoiceDeleteRetries(current => current.filter(item => item !== id));
      return value;
    },
    deleteCharacter: (id: string) =>
      deleteResource("character-delete", (signal) => client.deleteCharacter({ id }, signal)),
    previewVoice: async (voiceId: string, text: string) => {
      if (pendingRef.current) return null;
      previewAbort.current?.abort();
      setVoicePreview(null);
      setPreviewError(null);
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
      setModelNotice(null);

      const refreshed = await run("models", (signal) =>
        client.desktop({ type: "list_models" }, signal), false);
      if (refreshed?.type === "models" && mounted.current) setModels(refreshed.models);
      if (refreshed?.type === "error" && mounted.current) setActionError(refreshed.message);
      if (refreshed?.type === "models") await refreshInstalledModels(false);
      return imported;
    },
  };
}

export type ResourcesController = ReturnType<typeof useResourcesController>;
