import type { LlmModelsRequest, LlmModelsResult, LlmReasoningRequest, LlmReasoningResult, LlmSettings, LlmSettingsRequest, LlmSettingsSnapshot, LlmTestResult } from "@meowlive/contracts";
import { readServerError, ServerRequestError } from "./responses";
import { createAuthenticatedFetch } from "./auth";

export interface LlmClient {
  readonly baseUrl: string;
  getSettings(signal?: AbortSignal): Promise<LlmSettingsSnapshot>;
  saveSettings(request: LlmSettingsRequest, signal?: AbortSignal): Promise<LlmSettingsSnapshot>;
  testSettings(request: LlmSettingsRequest, signal?: AbortSignal): Promise<LlmTestResult>;
  listModels(request: LlmModelsRequest, signal?: AbortSignal): Promise<LlmModelsResult>;
  previewReasoning(request: LlmReasoningRequest, signal?: AbortSignal): Promise<LlmReasoningResult>;
}

export const reasoningEfforts = ["default", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"] as const;
const formats = ["openai_responses", "openai_chat", "anthropic_messages", "gemini_generate_content"];
const modes = ["cloud", "local"];
const object = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const exactKeys = (value: Record<string, unknown>, keys: string[]) => {
  const actual = Object.keys(value).sort();
  return actual.length === keys.length && actual.every((key, index) => key === [...keys].sort()[index]);
};
const utf8Length = (value: string) => new TextEncoder().encode(value).length;
const boundedText = (value: unknown, maxBytes: number, allowEmpty = false): value is string => typeof value === "string"
  && (allowEmpty || value.trim().length > 0) && utf8Length(value) <= maxBytes && !/[\u0000-\u001f\u007f]/u.test(value);
const integer = (value: unknown, min: number, max: number): value is number => Number.isSafeInteger(value) && (value as number) >= min && (value as number) <= max;
const invalid = () => new ServerRequestError("invalid_response", "主服务返回了无效的 LLM 配置。");

function readSettings(value: unknown): LlmSettings {
  if (!object(value) || !exactKeys(value, ["provider", "api_format", "base_url", "model", "mode", "timeout_seconds", "max_tokens", "json_mode", "reasoning_effort"])
    || !boundedText(value.provider, 64) || !formats.includes(value.api_format as string)
    || !boundedText(value.base_url, 4096, true) || !boundedText(value.model, 128, true)
    || !modes.includes(value.mode as string) || !integer(value.timeout_seconds, 1, 120)
    || !integer(value.max_tokens, 64, value.mode === "local" ? 1024 : 65536) || typeof value.json_mode !== "boolean"
    || !reasoningEfforts.includes(value.reasoning_effort as typeof reasoningEfforts[number])) throw invalid();
  return value as unknown as LlmSettings;
}

export function readLlmSnapshot(value: unknown): LlmSettingsSnapshot {
  if (!object(value) || !exactKeys(value, ["settings", "key_configured", "restart_required", "active_model", "storage_available"])
    || typeof value.key_configured !== "boolean" || typeof value.restart_required !== "boolean"
    || !boundedText(value.active_model, 128, true) || typeof value.storage_available !== "boolean") throw invalid();
  return { ...value, settings: readSettings(value.settings) } as LlmSettingsSnapshot;
}

function readTestResult(value: unknown): LlmTestResult {
  if (!object(value) || !exactKeys(value, ["message"]) || !boundedText(value.message, 500)) throw invalid();
  return { message: value.message };
}

function readModelsResult(value: unknown): LlmModelsResult {
  const invalidModels = () => new ServerRequestError("invalid_response", "主服务返回了无效的 LLM 模型列表。");
  if (!object(value) || !exactKeys(value, ["base_url", "models"]) || !boundedText(value.base_url, 4096)
    || !Array.isArray(value.models) || value.models.length > 1000) throw invalidModels();
  try {
    const url = new URL(value.base_url);
    if (!/^https?:$/u.test(url.protocol) || url.username || url.password || url.search || url.hash) throw invalidModels();
  } catch { throw invalidModels(); }
  const ids = new Set<string>();
  const models = value.models.map(model => {
    if (!object(model) || !exactKeys(model, ["id", "name"]) || !boundedText(model.id, 128)
      || !boundedText(model.name, 256) || ids.has(model.id)) throw invalidModels();
    ids.add(model.id);
    return { id: model.id, name: model.name };
  });
  return { base_url: value.base_url, models };
}

function readReasoningResult(value: unknown, request: LlmReasoningRequest): LlmReasoningResult {
  const invalidReasoning = () => new ServerRequestError("invalid_response", "主服务返回了无效的推理强度预览。");
  const rank = (effort: unknown) => reasoningEfforts.indexOf(effort as typeof reasoningEfforts[number]);
  if (!object(value) || !exactKeys(value, ["requested", "effective", "supported", "strategy", "budget_tokens", "note", "error"])
    || rank(value.requested) < 0 || value.requested !== request.reasoning_effort
    || !Array.isArray(value.supported) || value.supported.some((effort, index, values) => rank(effort) <= 0 || (index > 0 && rank(effort) <= rank(values[index - 1])))
    || (value.effective !== null && (value.effective === "default" ? value.requested !== "default" : rank(value.effective) <= 0 || !value.supported.includes(value.effective)))
    || (value.requested === "default" && value.effective !== null && value.effective !== "default")
    || (value.budget_tokens !== null && (value.effective === null || value.effective === "default" || !integer(value.budget_tokens, 1, Math.min(65024, request.max_tokens - 512))))
    || !boundedText(value.strategy, 128) || !boundedText(value.note, 4096, true)
    || (value.error !== null && !boundedText(value.error, 4096))) throw invalidReasoning();
  return value as unknown as LlmReasoningResult;
}

export function createLlmClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): LlmClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/u, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);

  async function request<T>(path: string, method: "GET" | "POST", read: (value: unknown) => T, signal?: AbortSignal, body?: LlmSettingsRequest | LlmModelsRequest | LlmReasoningRequest): Promise<T> {
    signal?.throwIfAborted();
    const controller = new AbortController();
    const cancel = () => controller.abort();
    signal?.addEventListener("abort", cancel, { once: true });
    let timedOut = false;
    const timeoutMs = options.timeoutMs ?? (path === "/api/llm/models" ? 32_000
      : path === "/api/llm/test" && body && "settings" in body ? body.settings.timeout_seconds * 1_000 + 2_000 : 12_000);
    const timer = setTimeout(() => { timedOut = true; controller.abort(); }, timeoutMs);
    const aborted = new Promise<never>((_resolve, reject) => {
      controller.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true });
    });
    try {
      const response = await Promise.race([fetcher(`${baseUrl}${path}`, {
        method,
        signal: controller.signal,
        ...(body ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) } : {}),
      }), aborted]);
      const value: unknown = await Promise.race([response.json().catch(() => null), aborted]);
      controller.signal.throwIfAborted();
      if (!response.ok) {
        const error = readServerError(value);
        throw new ServerRequestError(error?.code ?? "http_error", error?.message ?? `主服务请求失败（HTTP ${response.status}）。`, response.status);
      }
      return read(value);
    } catch (error) {
      if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
      if (timedOut) throw new ServerRequestError("request_timeout", "LLM 配置请求超时，请检查主服务状态。");
      if (error instanceof ServerRequestError) throw error;
      throw new ServerRequestError("connection_failed", "无法连接主服务，请检查服务地址和服务是否已启动。");
    } finally {
      clearTimeout(timer);
      signal?.removeEventListener("abort", cancel);
    }
  }

  return {
    baseUrl,
    getSettings: signal => request("/api/llm/settings", "GET", readLlmSnapshot, signal),
    saveSettings: (body, signal) => request("/api/llm/settings", "POST", readLlmSnapshot, signal, body),
    testSettings: (body, signal) => request("/api/llm/test", "POST", readTestResult, signal, body),
    listModels: (body, signal) => request("/api/llm/models", "POST", readModelsResult, signal, body),
    previewReasoning: (body, signal) => request("/api/llm/reasoning", "POST", value => readReasoningResult(value, body), signal, body),
  };
}
