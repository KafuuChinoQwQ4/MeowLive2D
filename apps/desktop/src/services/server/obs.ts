import type { ObsOperation, ObsSnapshot } from "@meowlive/contracts";
import { readServerError, ServerRequestError } from "./responses";

export interface ObsClient {
  getStatus(signal?: AbortSignal): Promise<ObsSnapshot>;
  execute(operation: ObsOperation, signal?: AbortSignal): Promise<ObsSnapshot>;
}

function sceneName(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0 && [...value].length <= 256 && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

function readSnapshot(value: unknown): ObsSnapshot {
  const data = value as Partial<ObsSnapshot> | null;
  if (!data || typeof data !== "object" || typeof data.connected !== "boolean" || typeof data.recording !== "boolean"
    || typeof data.current_scene !== "string" || !Array.isArray(data.scenes) || data.scenes.length > 256
    || !data.scenes.every(sceneName)
    || (data.connected ? !sceneName(data.current_scene) || !data.scenes.includes(data.current_scene) : data.recording || data.current_scene !== "" || data.scenes.length !== 0)) {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的 OBS 状态。");
  }
  return { connected: data.connected, recording: data.recording, current_scene: data.current_scene, scenes: data.scenes };
}

export function createObsClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): ObsClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/u, "");
  const fetcher = options.fetcher ?? ((...args: Parameters<typeof fetch>) => globalThis.fetch(...args));
  async function request(operation?: ObsOperation, signal?: AbortSignal): Promise<ObsSnapshot> {
    signal?.throwIfAborted();
    const controller = new AbortController();
    const cancel = () => controller.abort();
    signal?.addEventListener("abort", cancel, { once: true });
    let timedOut = false;
    const timer = setTimeout(() => { timedOut = true; controller.abort(); }, options.timeoutMs ?? 18_000);
    const aborted = new Promise<never>((_resolve, reject) => controller.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true }));
    try {
      const response = await Promise.race([fetcher(`${baseUrl}/api/obs`, {
        method: operation ? "POST" : "GET", signal: controller.signal,
        ...(operation ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(operation) } : {}),
      }), aborted]);
      const value: unknown = await Promise.race([response.json().catch(() => null), aborted]);
      controller.signal.throwIfAborted();
      if (!response.ok) {
        const error = readServerError(value);
        throw new ServerRequestError(error?.code ?? "http_error", error?.message ?? `OBS 请求失败（HTTP ${response.status}）。`, response.status);
      }
      return readSnapshot(value);
    } catch (error) {
      if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
      if (timedOut) throw new ServerRequestError("request_timeout", "OBS 操作超时。");
      if (error instanceof ServerRequestError) throw error;
      throw new ServerRequestError("connection_failed", "无法连接主服务或 OBS。");
    } finally {
      clearTimeout(timer);
      signal?.removeEventListener("abort", cancel);
    }
  }
  return { getStatus: (signal) => request(undefined, signal), execute: (operation, signal) => request(operation, signal) };
}
