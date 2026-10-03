import { createAuthenticatedFetch } from "./auth";
import { ServerRequestError } from "./responses";
import type { RuntimeLogEntry, RuntimeLogLevel, RuntimeLogListResponse } from "@meowlive/contracts";

export type { RuntimeLogEntry, RuntimeLogLevel, RuntimeLogListResponse };
export type RuntimeLogReport = { code: "browser_error" | "unhandled_rejection" | "request_failed" };
export interface RuntimeLogQuery { level?: RuntimeLogLevel; source?: string; category?: string; query?: string; limit?: number }

const localEntries: RuntimeLogEntry[] = [];
const MAX_LOCAL = 100;
const validLevel = (value: unknown): value is RuntimeLogLevel => value === "debug" || value === "info" || value === "warn" || value === "error";
const text = (value: unknown, max: number) => typeof value === "string" && value.length > 0 && value.length <= max && !/[\u0000-\u001f\u007f]/u.test(value);
function readEntry(value: unknown): RuntimeLogEntry | null {
  if (!value || typeof value !== "object") return null;
  const e = value as Record<string, unknown>;
  return text(e.id, 128) && text(e.timestamp, 64) && Number.isFinite(Date.parse(String(e.timestamp))) && validLevel(e.level)
    && text(e.source, 64) && text(e.category, 64) && text(e.code, 128) && text(e.summary, 1024)
    ? e as unknown as RuntimeLogEntry : null;
}
export function readRuntimeLogs(value: unknown): RuntimeLogListResponse {
  const body = value as Record<string, unknown> | null;
  if (!body || !Array.isArray(body.entries) || body.entries.length > 1000 || typeof body.storage_available !== "boolean" || typeof body.truncated !== "boolean") {
    throw new ServerRequestError("invalid_response", "服务返回了无效的日志列表。");
  }
  const entries = body.entries.map(readEntry);
  if (entries.some(entry => !entry)) throw new ServerRequestError("invalid_response", "服务返回了无效的日志记录。");
  return { entries: entries as RuntimeLogEntry[], storage_available: body.storage_available, truncated: body.truncated };
}
export function localRuntimeLogs(): RuntimeLogEntry[] { return [...localEntries]; }
export function filterRuntimeLogs(entries: RuntimeLogEntry[], query: RuntimeLogQuery): RuntimeLogEntry[] {
  const needle = query.query?.trim().toLowerCase();
  return entries.filter(entry => (!query.level || entry.level === query.level) && (!query.source || entry.source === query.source)
    && (!query.category || entry.category === query.category) && (!needle || `${entry.summary} ${entry.code} ${entry.source} ${entry.category}`.toLowerCase().includes(needle)))
    .sort((a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp)).slice(0, query.limit ?? 100);
}
export function runtimeLogParams(query: RuntimeLogQuery): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) if (value !== undefined && value !== "") params.set(key, String(value));
  return params.size ? `?${params}` : "";
}
export interface RuntimeLogClient {
  readonly baseUrl: string;
  list(query?: RuntimeLogQuery, signal?: AbortSignal): Promise<RuntimeLogListResponse>;
  report(report: RuntimeLogReport, signal?: AbortSignal): Promise<void>;
}
export async function requestRuntimeLogs(fetcher: typeof fetch, url: string, init: RequestInit, signal?: AbortSignal, timeoutMs = 5000): Promise<unknown> {
  signal?.throwIfAborted();
  const controller = new AbortController();
  const cancel = () => controller.abort();
  signal?.addEventListener("abort", cancel, { once: true });
  let timedOut = false;
  const timer = setTimeout(() => { timedOut = true; controller.abort(); }, timeoutMs);
  const aborted = new Promise<never>((_, reject) => controller.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true }));
  try {
    const response = await Promise.race([fetcher(url, { ...init, signal: controller.signal, cache: "no-store" }), aborted]);
    if (!response.ok) throw new ServerRequestError("http_error", `日志请求失败（HTTP ${response.status}）。`, response.status);
    const body = await Promise.race([response.text(), aborted]);
    if (body.length > 2_097_152) throw new ServerRequestError("invalid_response", "日志列表过大。");
    return body ? JSON.parse(body) as unknown : null;
  } catch (error) {
    if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
    if (timedOut) throw new ServerRequestError("request_timeout", "日志请求超时。");
    if (error instanceof ServerRequestError) throw error;
    throw new ServerRequestError("connection_failed", "无法读取日志，请检查服务状态。");
  } finally { clearTimeout(timer); signal?.removeEventListener("abort", cancel); }
}
export function createRuntimeLogClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): RuntimeLogClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/u, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);
  const reportedAt = new Map<string, number>();
  return {
    baseUrl,
    async list(query = {}, signal) {
      return readRuntimeLogs(await requestRuntimeLogs(fetcher, `${baseUrl}/api/logs${runtimeLogParams(query)}`, { method: "GET" }, signal, options.timeoutMs));
    },
    async report(report, signal) {
      const now = Date.now();
      if (now - (reportedAt.get(report.code) ?? -Infinity) < 5000) return;
      reportedAt.set(report.code, now);
      const safe: RuntimeLogEntry = { id: `local-${now}-${Math.random().toString(36).slice(2)}`, timestamp: new Date(now).toISOString(), level: report.code === "request_failed" ? "warn" : "error", source: "browser", category: report.code === "request_failed" ? "http" : "runtime", code: report.code, summary: report.code === "browser_error" ? "浏览器运行时错误" : report.code === "unhandled_rejection" ? "未处理的异步拒绝" : "服务请求失败" };
      localEntries.unshift(safe);
      if (localEntries.length > MAX_LOCAL) localEntries.length = MAX_LOCAL;
      try {
        await requestRuntimeLogs(fetcher, `${baseUrl}/api/logs`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code: report.code }) }, signal, options.timeoutMs);
        const index = localEntries.findIndex(entry => entry.id === safe.id);
        if (index >= 0) localEntries.splice(index, 1);
      } catch { /* Failed uploads remain observable in the bounded local buffer. */ }
    },
  };
}
