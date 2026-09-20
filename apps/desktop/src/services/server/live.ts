import type { LiveConnectionPhase, LiveConnectionSnapshot, LiveSettingsRequest, LiveSettingsSnapshot } from "@meowlive/contracts";
import { readServerError, ServerRequestError } from "./responses";
import { createAuthenticatedFetch } from "./auth";

export interface LiveClient {
  readonly baseUrl: string;
  getStatus(signal?: AbortSignal): Promise<LiveConnectionSnapshot>;
  connect(signal?: AbortSignal): Promise<LiveConnectionSnapshot>;
  disconnect(signal?: AbortSignal): Promise<LiveConnectionSnapshot>;
  getSettings(signal?: AbortSignal): Promise<LiveSettingsSnapshot>;
  saveSettings(request: LiveSettingsRequest, signal?: AbortSignal): Promise<LiveSettingsSnapshot>;
}

const phases: LiveConnectionPhase[] = [
  "disabled",
  "disconnected",
  "connecting",
  "connected",
  "reconnecting",
  "disconnecting",
  "failed",
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isUint32(value: unknown): value is number {
  return typeof value === "number"
    && Number.isFinite(value)
    && Number.isInteger(value)
    && value >= 0
    && value <= 0xffff_ffff;
}

function isBoundedText(value: unknown, maxLength: number): value is string {
  return typeof value === "string" && value.trim().length > 0 && [...value].length <= maxLength;
}

function isNullableBoundedText(value: unknown, maxLength: number): value is string | null {
  return value === null || isBoundedText(value, maxLength);
}

function readLiveSnapshot(value: unknown): LiveConnectionSnapshot {
  if (!isRecord(value)
    || !isBoundedText(value.platform, 32)
    || typeof value.configured !== "boolean"
    || !phases.includes(value.phase as LiveConnectionPhase)
    || !isNullableBoundedText(value.room_id, 128)
    || !isUint32(value.accepted_events)
    || !isUint32(value.duplicate_events)
    || !isUint32(value.rejected_events)
    || !isUint32(value.reconnect_attempts)
    || !isNullableBoundedText(value.last_error, 2_000)) {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的直播连接状态。");
  }
  return value as LiveConnectionSnapshot;
}

function readLiveSettings(value: unknown): LiveSettingsSnapshot {
  if (!isRecord(value)
    || typeof value.enabled !== "boolean"
    || typeof value.app_id !== "string"
    || (value.app_id !== "" && (!/^\d{1,19}$/u.test(value.app_id) || BigInt(value.app_id) > 9223372036854775807n))
    || typeof value.access_key_id_configured !== "boolean"
    || typeof value.access_key_secret_configured !== "boolean"
    || typeof value.identity_code_configured !== "boolean"
    || typeof value.storage_available !== "boolean") {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的直播配置。");
  }
  return value as LiveSettingsSnapshot;
}

export function createLiveClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): LiveClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);
  const timeoutMs = options.timeoutMs ?? 8_000;

  async function request<T>(path: string, method: "GET" | "POST", read: (value: unknown) => T, signal?: AbortSignal, body?: LiveSettingsRequest): Promise<T> {
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
        fetcher(`${baseUrl}${path}`, { method, signal: controller.signal,
          ...(body ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) } : {}),
        }),
        aborted,
      ]);
      const value: unknown = await Promise.race([response.json().catch(() => null), aborted]);
      controller.signal.throwIfAborted();
      if (!response.ok) {
        const error = readServerError(value);
        throw new ServerRequestError(
          error?.code ?? "http_error",
          error?.message ?? `主服务请求失败（HTTP ${response.status}）。`,
          response.status,
        );
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
    getStatus: (signal) => request("/api/live", "GET", readLiveSnapshot, signal),
    connect: (signal) => request("/api/live/connect", "POST", readLiveSnapshot, signal),
    disconnect: (signal) => request("/api/live/disconnect", "POST", readLiveSnapshot, signal),
    getSettings: (signal) => request("/api/live/settings", "GET", readLiveSettings, signal),
    saveSettings: (body, signal) => request("/api/live/settings", "POST", readLiveSettings, signal, body),
  };
}
