import type {
  AgentEventSnapshot,
  AgentEventStatus,
  AgentPhase,
  AgentSettings,
  AgentSnapshot,
  EventBatchRequest,
  EventBatchResult,
  LiveEventInput,
  PersonaProfileCreateRequest,
  PersonaProfileIdRequest,
  PersonaProfileRenameRequest,
  PersonaProfileUpdateRequest,
  PersonaProfilesSnapshot,
} from "@meowlive/contracts";
import { readServerError, ServerRequestError } from "./responses";
import { createAuthenticatedFetch } from "./auth";
import { isEventPayload } from "./eventPayload";

export interface AgentClient {
  readonly baseUrl: string;
  getStatus(signal?: AbortSignal): Promise<AgentSnapshot>;
  saveSettings(settings: AgentSettings, signal?: AbortSignal): Promise<AgentSnapshot>;
  getPersonaProfiles(signal?: AbortSignal): Promise<PersonaProfilesSnapshot>;
  createPersonaProfile(request: PersonaProfileCreateRequest, signal?: AbortSignal): Promise<PersonaProfilesSnapshot>;
  selectPersonaProfile(request: PersonaProfileIdRequest, signal?: AbortSignal): Promise<PersonaProfilesSnapshot>;
  renamePersonaProfile(request: PersonaProfileRenameRequest, signal?: AbortSignal): Promise<PersonaProfilesSnapshot>;
  updatePersonaProfile(request: PersonaProfileUpdateRequest, signal?: AbortSignal): Promise<PersonaProfilesSnapshot>;
  deletePersonaProfile(request: PersonaProfileIdRequest, signal?: AbortSignal): Promise<PersonaProfilesSnapshot>;
  pause(signal?: AbortSignal): Promise<AgentSnapshot>;
  resume(signal?: AbortSignal): Promise<AgentSnapshot>;
  submitEvents(batch: EventBatchRequest, signal?: AbortSignal): Promise<EventBatchResult>;
}

type RequestBody = AgentSettings | EventBatchRequest | PersonaProfileCreateRequest | PersonaProfileIdRequest | PersonaProfileRenameRequest | PersonaProfileUpdateRequest;

const phases: AgentPhase[] = ["paused", "waiting", "deciding", "speaking"];
const eventStatuses: AgentEventStatus[] = [
  "pending", "deciding", "skipped", "expired", "queued", "synthesizing",
  "ready", "playing", "completed", "cancelled", "failed", "unknown",
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isUint32(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && Number.isInteger(value) && value >= 0 && value <= 0xffff_ffff;
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function boundedText(value: unknown, min: number, max: number): value is string {
  return typeof value === "string" && (min === 0 || value.trim().length > 0)
    && [...value].length >= min && [...value].length <= max;
}

function isSettings(value: unknown): value is AgentSettings {
  return isRecord(value)
    && boundedText(value.persona, 1, 2000)
    && boundedText(value.system_prompt, 0, 4000)
    && boundedText(value.topic, 0, 200)
    && typeof value.proactive_enabled === "boolean"
    && isUint32(value.cooldown_ms) && value.cooldown_ms >= 1000 && value.cooldown_ms <= 3600000
    && isInteraction(value.interaction);
}

function isInteraction(value: unknown): boolean {
  if (!isRecord(value) || !["auto", "all", "selective"].includes(value.chat_read_mode as string)
    || typeof value.welcome_enabled !== "boolean") return false;
  return ([
    [value.busy_chat_count, 1, 1000], [value.busy_enter_count, 1, 1000],
    [value.busy_pending_count, 1, 512], [value.welcome_cooldown_ms, 1000, 3_600_000],
    [value.welcome_viewer_cooldown_ms, 1000, 86_400_000],
  ] as const).every(([count, min, max]) => isUint32(count) && count >= min && count <= max);
}

function isEvent(value: unknown): value is LiveEventInput {
  return isRecord(value)
    && boundedText(value.id, 1, 128)
    && boundedText(value.source, 1, 32)
    && boundedText(value.viewer, 1, 64)
    && isEventPayload(value.kind)
    && (value.gift_metadata === undefined || value.gift_metadata === null || value.kind.type === "gift");
}

function isEventSnapshot(value: unknown): value is AgentEventSnapshot {
  return isRecord(value)
    && isEvent(value.event)
    && eventStatuses.includes(value.status as AgentEventStatus)
    && isNullableString(value.speech_id)
    && isNullableString(value.error);
}

function readAgentSnapshot(value: unknown): AgentSnapshot {
  if (!isRecord(value)
    || typeof value.paused !== "boolean"
    || !phases.includes(value.phase as AgentPhase)
    || !isSettings(value.settings)
    || !Array.isArray(value.events)
    || value.events.length > 4_096
    || !value.events.every(isEventSnapshot)
    || !isNullableString(value.last_error)
    || !isNullableString(value.current_speech_id)
    || typeof value.llm_configured !== "boolean"
    || typeof value.bridge_connected !== "boolean") {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的 Agent 状态。");
  }
  return value as AgentSnapshot;
}

function readPersonaProfiles(value: unknown): PersonaProfilesSnapshot {
  if (!isRecord(value) || Object.keys(value).sort().join() !== "profiles,selected_profile_id,storage_available"
    || !Array.isArray(value.profiles) || value.profiles.length > 32
    || typeof value.storage_available !== "boolean") {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的人物卡配置。");
  }
  const ids = new Set<string>();
  for (const profile of value.profiles) {
    if (!isRecord(profile) || Object.keys(profile).sort().join() !== "id,name,persona"
      || !boundedText(profile.id, 1, 64) || !boundedText(profile.name, 1, 64)
      || !boundedText(profile.persona, 1, 2_000)
      || /[\u0000-\u001f\u007f]/u.test(profile.id) || /[\u0000-\u001f\u007f]/u.test(profile.name)
      || ids.has(profile.id)) {
      throw new ServerRequestError("invalid_response", "主服务返回了无效的人物卡配置。");
    }
    ids.add(profile.id);
  }
  if (value.selected_profile_id !== null && (typeof value.selected_profile_id !== "string" || !ids.has(value.selected_profile_id))) {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的人物卡配置。");
  }
  return value as PersonaProfilesSnapshot;
}

function readBatchResult(value: unknown): EventBatchResult {
  if (!isRecord(value)
    || !isUint32(value.accepted)
    || !isUint32(value.duplicates)
    || !(value.persisted === undefined || isUint32(value.persisted))
    || !(value.unscheduled === undefined || isUint32(value.unscheduled))) {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的事件接收结果。");
  }
  return {
    accepted: value.accepted,
    duplicates: value.duplicates,
    ...(value.persisted === undefined ? {} : { persisted: value.persisted }),
    ...(value.unscheduled === undefined ? {} : { unscheduled: value.unscheduled }),
  };
}

export function createAgentClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): AgentClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);
  const timeoutMs = options.timeoutMs ?? 8_000;

  async function request<T>(path: string, method: "GET" | "POST", read: (value: unknown) => T, signal?: AbortSignal, body?: RequestBody): Promise<T> {
    signal?.throwIfAborted();
    const controller = new AbortController();
    const cancel = () => controller.abort();
    signal?.addEventListener("abort", cancel, { once: true });
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, timeoutMs);
    const aborted = new Promise<never>((_resolve, reject) => {
      controller.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true });
    });

    try {
      const response = await Promise.race([
        fetcher(`${baseUrl}${path}`, {
          method,
          signal: controller.signal,
          ...(body ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) } : {}),
        }),
        aborted,
      ]);
      const value: unknown = await Promise.race([response.json().catch(() => null), aborted]);
      controller.signal.throwIfAborted();
      if (!response.ok) {
        const error = readServerError(value);
        throw new ServerRequestError(error?.code ?? "http_error", error?.message ?? `主服务请求失败（HTTP ${response.status}）。`, response.status);
      }
      return read(value);
    } catch (error) {
      if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
      if (timedOut) throw new ServerRequestError("request_timeout", "连接主服务超时，请检查服务状态。");
      if (error instanceof ServerRequestError) throw error;
      throw new ServerRequestError("connection_failed", "无法连接主服务，请检查服务地址和服务是否已启动。");
    } finally {
      clearTimeout(timer);
      signal?.removeEventListener("abort", cancel);
    }
  }

  return {
    baseUrl,
    getStatus: (signal) => request("/api/agent", "GET", readAgentSnapshot, signal),
    saveSettings: (body, signal) => request("/api/agent/settings", "POST", readAgentSnapshot, signal, body),
    getPersonaProfiles: (signal) => request("/api/agent/personas", "GET", readPersonaProfiles, signal),
    createPersonaProfile: (body, signal) => request("/api/agent/personas", "POST", readPersonaProfiles, signal, body),
    selectPersonaProfile: (body, signal) => request("/api/agent/personas/select", "POST", readPersonaProfiles, signal, body),
    renamePersonaProfile: (body, signal) => request("/api/agent/personas/rename", "POST", readPersonaProfiles, signal, body),
    updatePersonaProfile: (body, signal) => request("/api/agent/personas/update", "POST", readPersonaProfiles, signal, body),
    deletePersonaProfile: (body, signal) => request("/api/agent/personas/delete", "POST", readPersonaProfiles, signal, body),
    pause: (signal) => request("/api/agent/pause", "POST", readAgentSnapshot, signal),
    resume: (signal) => request("/api/agent/resume", "POST", readAgentSnapshot, signal),
    submitEvents: (body, signal) => request("/api/events", "POST", readBatchResult, signal, body),
  };
}
