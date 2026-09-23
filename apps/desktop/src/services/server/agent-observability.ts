import type { AgentSchedulerSnapshot, AgentTrace, AgentTraceList } from "@meowlive/contracts";
import { createAuthenticatedFetch } from "./auth";
import { ServerRequestError } from "./responses";

export interface AgentTraceQuery { limit?: number; before_ms?: number }
export interface AgentObservabilityClient {
  readonly baseUrl: string;
  getScheduler(signal?: AbortSignal): Promise<AgentSchedulerSnapshot>;
  listTraces(query?: AgentTraceQuery, signal?: AbortSignal): Promise<AgentTraceList>;
  getTrace(id: string, signal?: AbortSignal): Promise<AgentTrace>;
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
  return Object.keys(record).length === Object.keys(fields).length
    && Object.entries(fields).every(([key, check]) => Object.hasOwn(record, key) && check(record[key]));
};
const source: Check = value => {
  if (!text(4096)(value) || /[\\\s]/u.test(String(value))) return false;
  try {
    const parsed = new URL(String(value));
    return /^https?:$/u.test(parsed.protocol) && !parsed.username && !parsed.password;
  } catch { return false; }
};
const status = choice("running", "completed", "failed", "cancelled", "interrupted");
const usage = shape({ input_tokens: nullable(integer()), output_tokens: nullable(integer()), cache_read_tokens: nullable(integer()),
  cache_write_tokens: nullable(integer()), reasoning_tokens: nullable(integer()) });
const summary = shape({ id: text(128), status, trigger: text(64), started_at_ms: integer(), updated_at_ms: integer(),
  finished_at_ms: nullable(integer()), event_count: integer(), turn_count: integer(16), tool_count: integer(64),
  speech_id: nullable(text(128)), result: text(4096, true), truncated: bool });
const scheduler = shape({ ready: bool, phase: choice("paused", "waiting", "deciding", "speaking"),
  block_reason: choice("ready", "paused", "model_unavailable", "bridge_disconnected", "resource_changing", "gpu_busy",
    "synthesizer_busy", "receipt_backlog", "playback_busy", "decision_in_flight", "speech_in_flight", "cooldown",
    "completion_buffer_full", "work_id_exhausted", "no_eligible_events"), message: text(4096, true),
  remaining_ms: nullable(integer()), pending_events: integer(), deciding_events: integer(), active_speeches: integer(), updated_at_ms: integer() });
const traceList = shape({ traces: list(summary, 100), storage_available: bool, truncated: bool });
const traceEvent = shape({ id: text(128), kind: text(64), viewer: text(1024, true), summary: text(1024, true) });
const turn = shape({ id: text(128), index: integer(16, 1), tool_round: integer(3), retry_attempt: integer(1), provider: text(64, true),
  api_format: text(64, true), model: text(128, true), status, started_at_ms: integer(), first_token_ms: nullable(integer()),
  finished_at_ms: nullable(integer()), latency_ms: integer(), usage });
const step = shape({ sequence: integer(), occurred_at_ms: integer(),
  kind: choice("scheduled", "context_ready", "turn_started", "first_token", "turn_finished", "tool_started", "tool_finished",
    "decision_received", "validation_finished", "speech_queued", "speech_synthesizing", "speech_ready", "speech_playing", "trace_finished"),
  status: choice("running", "completed", "failed", "cancelled"), message: text(4096, true), turn_id: nullable(text(128)),
  tool_name: nullable(text(128)), speech_id: nullable(text(128)), elapsed_ms: nullable(integer()), sources: list(source, 32) });
const trace = shape({ summary, events: list(traceEvent, 16), turns: list(turn, 16), steps: list(step, 256) });

export function createAgentObservabilityClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): AgentObservabilityClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/u, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);
  async function request<T>(path: string, check: Check, label: string, signal?: AbortSignal): Promise<T> {
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
      const response = await Promise.race([fetcher(`${baseUrl}${path}`, { signal: controller.signal }), aborted]);
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
    } finally {
      clearTimeout(timer);
      signal?.removeEventListener("abort", cancel);
    }
  }
  return {
    baseUrl,
    getScheduler: signal => request("/api/agent/scheduler", scheduler, "调度状态", signal),
    listTraces: (query = {}, signal) => {
      const params = new URLSearchParams();
      if (query.limit !== undefined) params.set("limit", String(query.limit));
      if (query.before_ms !== undefined) params.set("before_ms", String(query.before_ms));
      return request(`/api/agent/traces${params.size ? `?${params}` : ""}`, traceList, "Trace 列表", signal);
    },
    getTrace: (id, signal) => request(`/api/agent/traces/${encodeURIComponent(id)}`, trace, "Trace 详情", signal),
  };
}
