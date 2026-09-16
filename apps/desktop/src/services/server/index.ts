/**
 * 主服务 HTTP 与 WebSocket 客户端边界。
 * 前端各 feature 通过此处访问后端；只消费 @meowlive/contracts 的协议类型。
 * 本模块不读取 LLM 或 TTS 的服务凭据，也不直连模型服务。
 */
import type { ServerStatus, SpeechRequest, SpeechSnapshot } from "@meowlive/contracts";
import { readServerError, readServerStatus, readSpeech, ServerRequestError } from "./responses";

export { ServerRequestError } from "./responses";

export interface ServerClient {
  readonly baseUrl: string;
  getStatus(signal?: AbortSignal): Promise<ServerStatus>;
  submitSpeech(request: SpeechRequest, signal?: AbortSignal): Promise<SpeechSnapshot>;
  stop(signal?: AbortSignal): Promise<ServerStatus>;
}

export function createServerClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): ServerClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/, "");
  const fetcher = options.fetcher ?? globalThis.fetch.bind(globalThis);
  const timeoutMs = options.timeoutMs ?? 8_000;

  async function request<T>(path: string, method: "GET" | "POST", read: (value: unknown) => T, signal?: AbortSignal, body?: SpeechRequest): Promise<T> {
    signal?.throwIfAborted();
    const controller = new AbortController();
    const cancel = () => controller.abort();
    signal?.addEventListener("abort", cancel, { once: true });
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, timeoutMs);

    try {
      const response = await fetcher(`${baseUrl}${path}`, {
        method,
        signal: controller.signal,
        ...(body ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) } : {}),
      });
      const value: unknown = await response.json().catch(() => null);
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
    getStatus: (signal) => request("/api/status", "GET", readServerStatus, signal),
    submitSpeech: (body, signal) => request("/api/speech", "POST", readSpeech, signal, body),
    stop: (signal) => request("/api/stop", "POST", readServerStatus, signal),
  };
}
