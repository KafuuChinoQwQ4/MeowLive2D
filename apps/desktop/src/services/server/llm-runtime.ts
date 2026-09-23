import type { AgentActivitySnapshot, AgentRuntimeSettingsRequest, AgentRuntimeSettingsSnapshot, LlmUsageSnapshot } from "@meowlive/contracts";
import { createAuthenticatedFetch } from "./auth";
import { ServerRequestError } from "./responses";

export interface LlmUsageQuery { since_ms?: number; until_ms?: number; provider?: string; model?: string }
export interface LlmRuntimeClient {
  readonly baseUrl: string;
  getSettings(signal?: AbortSignal): Promise<AgentRuntimeSettingsSnapshot>;
  saveSettings(request: AgentRuntimeSettingsRequest, signal?: AbortSignal): Promise<AgentRuntimeSettingsSnapshot>;
  getUsage(query?: LlmUsageQuery, signal?: AbortSignal): Promise<LlmUsageSnapshot>;
  getActivity(signal?: AbortSignal): Promise<AgentActivitySnapshot>;
}

type Check = (value: unknown) => boolean;
const text = (max: number, empty = false): Check => value => typeof value === "string" && (empty || value.trim().length > 0)
  && new TextEncoder().encode(value).length <= max && !/[\u0000-\u001f\u007f]/u.test(value);
const integer = (max = Number.MAX_SAFE_INTEGER, min = 0): Check => value => Number.isSafeInteger(value) && Number(value) >= min && Number(value) <= max;
const bool: Check = value => typeof value === "boolean";
const nullable = (check: Check): Check => value => value === null || check(value);
const choice = (...values: string[]): Check => value => typeof value === "string" && values.includes(value);
const list = (check: Check, max: number): Check => value => Array.isArray(value) && value.length <= max && value.every(check);
const shape = (fields: Record<string, Check>): Check => value => {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  return Object.keys(record).length === Object.keys(fields).length && Object.entries(fields).every(([key, check]) => Object.hasOwn(record, key) && check(record[key]));
};
const shapeWithOptional = (fields: Record<string, Check>, optional: Record<string, Check>): Check => value => {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  const allowed = new Set([...Object.keys(fields), ...Object.keys(optional)]);
  return Object.keys(record).every(key => allowed.has(key))
    && Object.entries(fields).every(([key, check]) => Object.hasOwn(record, key) && check(record[key]))
    && Object.entries(optional).every(([key, check]) => !Object.hasOwn(record, key) || check(record[key]));
};
const url = (empty = false, source = false): Check => value => {
  if (!text(4096, empty)(value) || /[\\\s]/u.test(String(value))) return false;
  if (value === "" && empty) return true;
  try {
    const parsed = new URL(String(value));
    return /^https?:$/u.test(parsed.protocol) && !parsed.username && !parsed.password && (source || (!parsed.search && !parsed.hash));
  } catch { return false; }
};
export function validRuntimeSearchEndpoint(provider: string, endpoint: string): boolean {
  if (!text(2048)(endpoint) || !url()(endpoint)) return false;
  const parsed = new URL(endpoint);
  return parsed.protocol === "https:" || provider === "searxng" && (parsed.hostname === "localhost" || parsed.hostname === "[::1]" || /^127(?:\.\d{1,3}){3}$/u.test(parsed.hostname));
}
const amount: Check = value => typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1_000_000;
const usage = shape({ input_tokens: nullable(integer()), output_tokens: nullable(integer()), cache_read_tokens: nullable(integer()),
  cache_write_tokens: nullable(integer()), reasoning_tokens: nullable(integer()) });
const totals = shape(Object.fromEntries(["calls", "input_tokens", "output_tokens", "cache_read_tokens", "cache_write_tokens", "reasoning_tokens",
  "estimated_cost_microusd", "unpriced_calls", "unknown_usage_calls"].map(key => [key, integer()])));
const identity = { provider: text(64), base_url: url(true), model: text(128) };
const price = shape({ ...identity, input_usd_per_million: amount, output_usd_per_million: amount,
  cache_read_usd_per_million: amount, cache_write_usd_per_million: amount });
const settingsSnapshotShape = shape({ search_key_configured: bool, storage_available: bool, settings: shape({
  cache_enabled: bool, streaming: bool, tools_enabled: bool, environment_enabled: bool, web_search_enabled: bool,
  search_provider: choice("brave", "searxng"), search_endpoint: url(), max_tool_rounds: integer(3, 1), tool_timeout_seconds: integer(8, 1), prices: list(price, 64),
}) });
const settingsSnapshot: Check = value => {
  if (!settingsSnapshotShape(value)) return false;
  const settings = (value as AgentRuntimeSettingsSnapshot).settings;
  return validRuntimeSearchEndpoint(settings.search_provider, settings.search_endpoint);
};
const usageSnapshot = shape({ totals, storage_available: bool, truncated: bool, groups: list(shape({ ...identity, totals }), Infinity),
  records: list(shapeWithOptional({ ...identity, id: text(128), started_at_ms: integer(), api_format: text(64), operation: text(128),
    status: choice("running", "completed", "failed", "cancelled", "interrupted"), latency_ms: integer(), first_token_ms: nullable(integer()),
    usage, estimated_cost_microusd: nullable(integer()), }, { trace_id: text(128), turn_id: text(128), tool_round: integer(3), retry_attempt: integer(1) }), 200) });
const activitySnapshot = shape({ run_id: nullable(text(128)), phase: choice("idle", "thinking", "receiving", "tool", "completed", "failed", "cancelled"),
  started_at_ms: nullable(integer()), updated_at_ms: integer(), output_characters: integer(), tool_round: integer(3), message: text(4096, true),
  tools: list(shape({ name: text(128), status: choice("running", "completed", "failed"), elapsed_ms: integer(), sources: list(url(false, true), 32) }), 32) });

export function createLlmRuntimeClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): LlmRuntimeClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/u, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);
  async function request<T>(path: string, check: Check, label: string, signal?: AbortSignal, body?: AgentRuntimeSettingsRequest): Promise<T> {
    signal?.throwIfAborted();
    const controller = new AbortController();
    const cancel = () => controller.abort();
    signal?.addEventListener("abort", cancel, { once: true });
    let timedOut = false;
    const timer = setTimeout(() => { timedOut = true; controller.abort(); }, options.timeoutMs ?? 12_000);
    const aborted = new Promise<never>((_resolve, reject) => {
      controller.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true });
    });
    try {
      const response = await Promise.race([fetcher(`${baseUrl}${path}`, { method: body ? "POST" : "GET", signal: controller.signal,
        ...(body ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) } : {}) }), aborted]);
      const value: unknown = await Promise.race([response.json().catch(() => null), aborted]);
      controller.signal.throwIfAborted();
      if (!response.ok) throw new ServerRequestError("http_error", `${label}请求失败，请检查服务状态与配置（HTTP ${response.status}）。`, response.status);
      if (!check(value)) throw new ServerRequestError("invalid_response", `主服务返回了无效的${label}。请确认主服务与面板版本一致。`);
      return value as T;
    } catch (error) {
      if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
      if (timedOut) throw new ServerRequestError("request_timeout", `${label}请求超时，请检查主服务状态。`);
      if (error instanceof ServerRequestError) throw error;
      throw new ServerRequestError("connection_failed", "无法连接主服务，请检查服务地址和服务是否已启动。");
    } finally { clearTimeout(timer); signal?.removeEventListener("abort", cancel); }
  }
  return {
    baseUrl,
    getSettings: signal => request("/api/agent/runtime", settingsSnapshot, "运行配置", signal),
    saveSettings: (body, signal) => request("/api/agent/runtime", settingsSnapshot, "运行配置", signal, body),
    getActivity: signal => request("/api/agent/activity", activitySnapshot, "Agent 活动", signal),
    getUsage: (query = {}, signal) => {
      const params = new URLSearchParams();
      for (const [key, value] of Object.entries(query)) if (value !== undefined && value !== "") params.set(key, String(value));
      return request(`/api/llm/usage${params.size ? `?${params}` : ""}`, usageSnapshot, "用量统计", signal);
    },
  };
}
