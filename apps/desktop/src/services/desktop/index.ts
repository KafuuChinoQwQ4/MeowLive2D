/**
 * Tauri 桌面命令客户端边界。
 * 浏览器开发模式与桌面可用能力在此区分；各 feature 不直接导入 Tauri SDK。
 */
import type { RuntimeLogEntry, RuntimeLogListResponse } from "@meowlive/contracts";
import { readRuntimeLogs } from "../server/logs";

export interface DesktopStatus {
  config_path: string;
  server_url: string;
  server: { ready: boolean; managed: boolean; last_error: string | null; log_path: string };
  runtime: { running: boolean; simulation: boolean; last_error: string | null };
  runtime_logs?: RuntimeLogListResponse;
}

export function readDesktopStatus(value: unknown): DesktopStatus {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid desktop status");
  const status = value as Record<string, unknown>;
  const runtime = status.runtime as Record<string, unknown> | undefined;
  const server = status.server as Record<string, unknown> | undefined;
  if (!server || typeof server.ready !== "boolean" || typeof server.managed !== "boolean" || typeof server.log_path !== "string" || !(server.last_error === null || typeof server.last_error === "string")) throw new Error("Invalid desktop server status");
  if (typeof status.config_path !== "string" || typeof status.server_url !== "string" || !validOrigin(status.server_url) || !runtime || typeof runtime.running !== "boolean" || typeof runtime.simulation !== "boolean" || !(runtime.last_error === null || typeof runtime.last_error === "string")) throw new Error("Invalid desktop status");
  return { config_path: status.config_path, server_url: status.server_url, server: server as DesktopStatus["server"], runtime: runtime as DesktopStatus["runtime"],
    ...(status.runtime_logs === undefined ? {} : { runtime_logs: readRuntimeLogs(status.runtime_logs) }) };
}

function validOrigin(value: string): boolean {
  try {
    const url = new URL(value);
    return value.length <= 4096 && !/\s/u.test(value) && ["http:", "https:"].includes(url.protocol)
      && !url.username && !url.password && !url.search && !url.hash && url.pathname === "/"
      && /^https?:\/\/[^/?#]+$/u.test(value);
  } catch { return false; }
}

const desktopEvents: RuntimeLogEntry[] = [];
let observedDesktop = "";
let previousServerState = "";
let previousRuntimeState = "";
let desktopTruncated = false;
let eventSequence = 0;
const desktopSummaries: Record<string, string> = {
  desktop_server_ready: "桌面主服务已就绪", desktop_server_failed: "桌面主服务启动或运行失败",
  desktop_server_waiting: "桌面主服务尚未就绪", desktop_runtime_running: "桌面执行端正在运行",
  desktop_runtime_failed: "桌面执行端运行失败", desktop_runtime_stopped: "桌面执行端已停止",
};
/** Older native shells expose status only. Observe transitions without retaining error text. */
function observeDesktopLogs(status: DesktopStatus): RuntimeLogListResponse {
  const identity = `${status.config_path}\n${status.server_url}`;
  if (identity !== observedDesktop) {
    observedDesktop = identity; previousServerState = ""; previousRuntimeState = "";
    desktopEvents.length = 0; desktopTruncated = false;
  }
  const serverState = status.server.ready ? "ready" : status.server.last_error ? "failed" : "waiting";
  const runtimeState = status.runtime.running ? "running" : status.runtime.last_error ? "failed" : "stopped";
  for (const [part, state, previous] of [["server", serverState, previousServerState], ["runtime", runtimeState, previousRuntimeState]]) {
    if (state === previous) continue;
    const code = `desktop_${part}_${state}`;
    desktopEvents.unshift({ id: `desktop-${++eventSequence}`, timestamp: new Date().toISOString(), source: "desktop", category: part === "server" ? "server" : "execution",
      level: state === "failed" ? "error" : state === "stopped" ? "warn" : "info", code, summary: desktopSummaries[code] });
    if (desktopEvents.length > 100) { desktopEvents.length = 100; desktopTruncated = true; }
  }
  previousServerState = serverState; previousRuntimeState = runtimeState;
  return { entries: [...desktopEvents], storage_available: false, truncated: desktopTruncated };
}

export async function getDesktopStatus(): Promise<DesktopStatus | null> {
  if (!("__TAURI_INTERNALS__" in globalThis)) return null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const value = await Promise.race([
      import("@tauri-apps/api/core").then(({ invoke }) => invoke("desktop_status")),
      new Promise<never>((_resolve, reject) => { timer = setTimeout(() => reject(new Error("桌面状态读取超时")), 5_000); }),
    ]);
    const status = readDesktopStatus(value);
    return { ...status, runtime_logs: status.runtime_logs ?? observeDesktopLogs(status) };
  } finally { clearTimeout(timer); }
}
