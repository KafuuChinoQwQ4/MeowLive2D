import type {
  CharacterPreviewRequest,
  CharacterProfile,
  CharacterSaveRequest,
  DesktopResourceOperation,
  DesktopResourceResult,
  ImportedModel,
  ResourceSelection,
  ResourceSnapshot,
  VoiceCreateRequest,
  VoiceProfile,
  VtsHotkey,
  VtsModel,
} from "@meowlive/contracts";

import { readServerError, ServerRequestError } from "./responses";
import { createAuthenticatedFetch } from "./auth";

const DEFAULT_BASE_URL = "http://127.0.0.1:19600";
const DEFAULT_TIMEOUT_MS = 8_000;
const MAX_VOICES = 64;
const MAX_CHARACTERS = 64;
const MAX_MAPPINGS = 32;
const MAX_DESKTOP_ITEMS = 256;

type Fetcher = typeof fetch;

export interface ResourcesClientOptions {
  baseUrl?: string;
  fetcher?: Fetcher;
  timeoutMs?: number;
}

export interface ResourcesClient {
  readonly baseUrl: string;
  getSnapshot(signal?: AbortSignal): Promise<ResourceSnapshot>;
  createVoice(
    metadata: VoiceCreateRequest,
    audio: File,
    signal?: AbortSignal,
  ): Promise<ResourceSnapshot>;
  selectVoice(selection: ResourceSelection, signal?: AbortSignal): Promise<ResourceSnapshot>;
  deleteVoice(selection: ResourceSelection, signal?: AbortSignal): Promise<ResourceSnapshot>;
  deleteCharacter(selection: ResourceSelection, signal?: AbortSignal): Promise<ResourceSnapshot>;
  saveCharacter(request: CharacterSaveRequest, signal?: AbortSignal): Promise<ResourceSnapshot>;
  selectCharacter(selection: ResourceSelection, signal?: AbortSignal): Promise<ResourceSnapshot>;
  previewCharacter(
    request: CharacterPreviewRequest,
    signal?: AbortSignal,
  ): Promise<ResourceSnapshot>;
  desktop(operation: DesktopResourceOperation, signal?: AbortSignal): Promise<DesktopResourceResult>;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isUint32(value: unknown, minimum = 0): value is number {
  return Number.isInteger(value) && (value as number) >= minimum && (value as number) <= 0xffff_ffff;
}

function isText(value: unknown, maximum: number, allowEmpty = false): value is string {
  return (
    typeof value === "string" &&
    Array.from(value).length <= maximum &&
    (allowEmpty || value.trim().length > 0) &&
    !/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/u.test(value)
  );
}

function isIdentifier(value: unknown, maximum = 128): value is string {
  return isText(value, maximum) && !/[\\/]/u.test(value);
}

function isPortableId(value: unknown): value is string {
  return typeof value === "string" && /^[A-Za-z0-9_-]{1,64}$/u.test(value);
}

function parseVoice(value: unknown): VoiceProfile | null {
  if (!isObject(value)) return null;
  const available = value.available;
  const error = value.error;
  if (
    !isPortableId(value.id) ||
    value.id === "default" ||
    !isText(value.name, 80) ||
    !["zh", "en", "ja", "ko", "yue", "auto"].includes(String(value.language)) ||
    !isText(value.reference_text, 500) ||
    !isUint32(value.duration_ms, 3_000) ||
    value.duration_ms > 10_000 ||
    !isUint32(value.sample_rate, 8_000) ||
    value.sample_rate > 48_000 ||
    ![1, 2].includes(value.channels as number) ||
    typeof available !== "boolean" ||
    !((available && error === null) || (!available && isText(error, 500)))
  ) {
    return null;
  }
  return value as unknown as VoiceProfile;
}

function parseCharacter(value: unknown, voiceIds: Set<string>): CharacterProfile | null {
  if (!isObject(value) || !Array.isArray(value.mappings) || value.mappings.length > MAX_MAPPINGS) {
    return null;
  }
  if (
    !isPortableId(value.id) ||
    !isText(value.name, 80) ||
    !isText(value.model_id, 128) ||
    !(value.voice_id === "" || voiceIds.has(value.voice_id as string)) ||
    typeof value.mouth_parameter !== "string" ||
    !/^[A-Za-z0-9]{4,32}$/u.test(value.mouth_parameter)
  ) {
    return null;
  }

  const intents = new Set<string>();
  for (const mapping of value.mappings) {
    if (
      !isObject(mapping) ||
      !isText(mapping.intent, 40) ||
      !isText(mapping.hotkey_id, 128) ||
      !(mapping.fallback_hotkey_id === null || isText(mapping.fallback_hotkey_id, 128)) ||
      typeof mapping.validated !== "boolean" ||
      intents.has(mapping.intent)
    ) {
      return null;
    }
    intents.add(mapping.intent);
  }
  return value as unknown as CharacterProfile;
}

function parseSnapshot(value: unknown): ResourceSnapshot {
  if (
    !isObject(value) ||
    !Array.isArray(value.voices) ||
    value.voices.length > MAX_VOICES ||
    !Array.isArray(value.characters) ||
    value.characters.length > MAX_CHARACTERS ||
    typeof value.default_voice_available !== "boolean"
  ) {
    throw new ServerRequestError("invalid_response", "资源服务返回了无效数据");
  }

  const voices: VoiceProfile[] = [];
  const voiceIds = new Set<string>(["default"]);
  for (const item of value.voices) {
    const voice = parseVoice(item);
    if (!voice || voiceIds.has(voice.id)) {
      throw new ServerRequestError("invalid_response", "资源服务返回了无效音色数据");
    }
    voiceIds.add(voice.id);
    voices.push(voice);
  }

  const characters: CharacterProfile[] = [];
  const characterIds = new Set<string>();
  for (const item of value.characters) {
    const character = parseCharacter(item, voiceIds);
    if (!character || characterIds.has(character.id)) {
      throw new ServerRequestError("invalid_response", "资源服务返回了无效角色数据");
    }
    characterIds.add(character.id);
    characters.push(character);
  }

  if (
    !(value.active_voice_id === "" || voiceIds.has(value.active_voice_id as string)) ||
    !(value.active_character_id === null || characterIds.has(value.active_character_id as string))
  ) {
    throw new ServerRequestError("invalid_response", "资源服务返回了无效的当前选择");
  }

  return {
    voices,
    characters,
    active_voice_id: value.active_voice_id as string,
    active_character_id: value.active_character_id as string | null,
    default_voice_available: value.default_voice_available,
  };
}

function parseModels(value: unknown): VtsModel[] | null {
  if (!Array.isArray(value) || value.length > MAX_DESKTOP_ITEMS) return null;
  const ids = new Set<string>();
  const models: VtsModel[] = [];
  for (const item of value) {
    if (!isObject(item) || !isText(item.id, 128) || !isText(item.name, 256) || ids.has(item.id)) {
      return null;
    }
    ids.add(item.id);
    models.push(item as unknown as VtsModel);
  }
  return models;
}

function parseHotkeys(value: unknown): VtsHotkey[] | null {
  if (!Array.isArray(value) || value.length > MAX_DESKTOP_ITEMS) return null;
  const ids = new Set<string>();
  const hotkeys: VtsHotkey[] = [];
  for (const item of value) {
    if (!isObject(item) || !isText(item.id, 128) || !isText(item.name, 256) || ids.has(item.id)) {
      return null;
    }
    ids.add(item.id);
    hotkeys.push(item as unknown as VtsHotkey);
  }
  return hotkeys;
}

function parseDesktopResult(value: unknown): DesktopResourceResult {
  if (!isObject(value) || typeof value.type !== "string") {
    throw new ServerRequestError("invalid_response", "桌面资源服务返回了无效数据");
  }

  switch (value.type) {
    case "models": {
      const models = parseModels(value.models);
      if (models) return { type: "models", models };
      break;
    }
    case "imported_models": {
      if (parseModels(value.models) && Array.isArray(value.models)
        && value.models.every((model: unknown) => isObject(model)
          && typeof model.id === "string" && /^[a-f0-9]{64}$/u.test(model.id)
          && (model.model_id === null || isText(model.model_id, 128)))) {
        return { type: "imported_models", models: value.models as ImportedModel[] };
      }
      break;
    }
    case "model_deleted":
      if (typeof value.id === "string" && /^[a-f0-9]{64}$/u.test(value.id)
        && typeof value.restart_required === "boolean") return value as unknown as DesktopResourceResult;
      break;
    case "model_loaded":
      if (isText(value.model_id, 128)) return value as unknown as DesktopResourceResult;
      break;
    case "hotkeys": {
      const hotkeys = parseHotkeys(value.hotkeys);
      if (isText(value.model_id, 128) && hotkeys) {
        return { type: "hotkeys", model_id: value.model_id, hotkeys };
      }
      break;
    }
    case "hotkey_triggered":
      if (isText(value.hotkey_id, 128)) return value as unknown as DesktopResourceResult;
      break;
    case "model_imported":
      if (
        isText(value.model_name, 256) &&
        isIdentifier(value.model_file, 256) &&
        isUint32(value.files, 1) &&
        isUint32(value.bytes, 1) &&
        typeof value.restart_required === "boolean"
      ) {
        return value as unknown as DesktopResourceResult;
      }
      break;
    case "error":
      if (isText(value.code, 128) && isText(value.message, 1_000)) {
        return value as unknown as DesktopResourceResult;
      }
      break;
  }
  throw new ServerRequestError("invalid_response", "桌面资源服务返回了无效数据");
}

function abortReason(signal: AbortSignal): unknown {
  return signal.reason ?? new DOMException("The operation was aborted", "AbortError");
}

async function requestJson(
  fetcher: Fetcher,
  timeoutMs: number,
  url: string,
  init: RequestInit,
  signal: AbortSignal | undefined,
): Promise<unknown> {
  if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
  const controller = new AbortController();
  let timedOut = false;
  const timeoutId = setTimeout(() => {
    timedOut = true;
    controller.abort(new DOMException("Request timed out", "TimeoutError"));
  }, timeoutMs);
  const onAbort = () => controller.abort(abortReason(signal as AbortSignal));
  signal?.addEventListener("abort", onAbort, { once: true });
  if (signal?.aborted) onAbort();

  const aborted = new Promise<never>((_, reject) => {
    const rejectAbort = () => reject(abortReason(controller.signal));
    controller.signal.addEventListener("abort", rejectAbort, { once: true });
    if (controller.signal.aborted) rejectAbort();
  });

  try {
    const response = await Promise.race([
      fetcher(url, { ...init, signal: controller.signal }),
      aborted,
    ]);
    const value = await Promise.race([
      response.json().catch(() => null),
      aborted,
    ]);
    if (!response.ok) {
      const error = readServerError(value);
      throw new ServerRequestError(
        error?.code ?? "http_error",
        error?.message ?? `主服务请求失败（HTTP ${response.status}）。`,
        response.status,
      );
    }
    try {
      if (value === null) throw new Error("invalid JSON");
      return value;
    } catch {
      throw new ServerRequestError("invalid_response", "服务返回的内容不是有效 JSON");
    }
  } catch (error) {
    if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
    if (timedOut) throw new ServerRequestError("request_timeout", "连接主服务超时，请检查服务状态。");
    if (error instanceof ServerRequestError) throw error;
    throw new ServerRequestError("connection_failed", "无法连接主服务，请检查服务地址和服务是否已启动。");
  } finally {
    clearTimeout(timeoutId);
    signal?.removeEventListener("abort", onAbort);
  }
}

function jsonRequest(body: unknown): RequestInit {
  return {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  };
}

export function createResourceClient(options: ResourcesClientOptions = {}): ResourcesClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? DEFAULT_BASE_URL).replace(/\/+$/u, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const request = (path: string, init: RequestInit, signal?: AbortSignal) =>
    requestJson(fetcher, timeoutMs, `${baseUrl}${path}`, init, signal);

  return {
    baseUrl,
    async getSnapshot(signal) {
      return parseSnapshot(await request("/api/resources", { method: "GET" }, signal));
    },
    async createVoice(metadata, audio, signal) {
      const form = new FormData();
      form.append("metadata", JSON.stringify(metadata));
      form.append("audio", audio);
      return parseSnapshot(await request("/api/voices", { method: "POST", body: form }, signal));
    },
    async selectVoice(selection, signal) {
      return parseSnapshot(await request("/api/voices/select", jsonRequest(selection), signal));
    },
    async deleteVoice(selection, signal) {
      return parseSnapshot(await request("/api/voices/delete", jsonRequest(selection), signal));
    },
    async deleteCharacter(selection, signal) {
      return parseSnapshot(await request("/api/characters/delete", jsonRequest(selection), signal));
    },
    async saveCharacter(character, signal) {
      return parseSnapshot(await request("/api/characters/save", jsonRequest(character), signal));
    },
    async selectCharacter(selection, signal) {
      return parseSnapshot(await request("/api/characters/select", jsonRequest(selection), signal));
    },
    async previewCharacter(preview, signal) {
      return parseSnapshot(await request("/api/characters/preview", jsonRequest(preview), signal));
    },
    async desktop(operation, signal) {
      const result = parseDesktopResult(
        await request("/api/desktop/resources", jsonRequest(operation), signal),
      );
      const matches = result.type === "error"
        || (operation.type === "list_imported_models" && result.type === "imported_models")
        || (operation.type === "delete_imported_model" && result.type === "model_deleted" && operation.id === result.id)
        || (operation.type === "list_models" && result.type === "models")
        || (operation.type === "list_hotkeys" && result.type === "hotkeys" && operation.model_id === result.model_id)
        || (operation.type === "load_model" && result.type === "model_loaded" && operation.model_id === result.model_id)
        || (operation.type === "trigger_hotkey" && result.type === "hotkey_triggered"
          && [operation.hotkey_id, operation.fallback_hotkey_id].includes(result.hotkey_id))
        || (operation.type === "import_model" && result.type === "model_imported");
      if (!matches) throw new ServerRequestError("invalid_response", "桌面资源结果与请求不匹配");
      return result;
    },
  };
}
