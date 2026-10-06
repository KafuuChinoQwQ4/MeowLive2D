/**
 * Tauri 桌面命令客户端边界。
 * 浏览器开发模式与桌面可用能力在此区分；各 feature 不直接导入 Tauri SDK。
 */
import { readEnvironmentSnapshot, type EnvironmentSnapshot } from "./environment";
import type { LauncherServiceId, RuntimeLogEntry, RuntimeLogListResponse } from "@meowlive/contracts";
import { readRuntimeLogs } from "../server/logs";
import { browserDesktopStatus, invokeDesktop } from "./transport";

export interface DesktopStatus {
  platform?: "windows" | "linux";
  config_path: string;
  server_url: string;
  environment?: EnvironmentSnapshot;
  server: { stopped?: boolean; ready: boolean; managed: boolean; last_error: string | null; log_path: string };
  runtime: { running: boolean; simulation: boolean; last_error: string | null };
  runtime_logs?: RuntimeLogListResponse;
  browser_panel_error?: string | null;
}

export function readDesktopStatus(value: unknown): DesktopStatus {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid desktop status");
  const status = value as Record<string, unknown>;
  const runtime = status.runtime as Record<string, unknown> | undefined;
  const server = status.server as Record<string, unknown> | undefined;
  if (server?.stopped !== undefined && typeof server.stopped !== "boolean") throw new Error("Invalid desktop server status");
  if (!server || typeof server.ready !== "boolean" || typeof server.managed !== "boolean" || typeof server.log_path !== "string" || !(server.last_error === null || typeof server.last_error === "string")) throw new Error("Invalid desktop server status");
  const platform = status.platform;
  if (status.browser_panel_error !== undefined && status.browser_panel_error !== null && typeof status.browser_panel_error !== "string") throw new Error("Invalid browser panel status");
  if (platform !== undefined && platform !== "windows" && platform !== "linux") throw new Error("Invalid desktop platform");
  if (typeof status.config_path !== "string" || typeof status.server_url !== "string" || !validOrigin(status.server_url) || (!runtime && platform !== "linux") || (runtime && (typeof runtime.running !== "boolean" || typeof runtime.simulation !== "boolean" || !(runtime.last_error === null || typeof runtime.last_error === "string")))) throw new Error("Invalid desktop status");
  return { ...(platform === undefined ? {} : { platform }), config_path: status.config_path, server_url: status.server_url, server: server as DesktopStatus["server"], runtime: (runtime ?? { running: false, simulation: false, last_error: null }) as DesktopStatus["runtime"],
    ...(status.environment === undefined ? {} : { environment: readEnvironmentSnapshot(status.environment) }),
    ...(status.runtime_logs === undefined ? {} : { runtime_logs: readRuntimeLogs(status.runtime_logs) }),
    ...(status.browser_panel_error === undefined ? {} : { browser_panel_error: status.browser_panel_error as string | null }) };
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
  desktop_wsl_timeout: "WSL 控制服务连接超时（Wsl/Service/0x8007274c）。保存 WSL 工作后，在 Windows PowerShell 执行 wsl --shutdown，再重试主服务开关。",
  desktop_wsl_project_missing: "所选 WSL 发行版中找不到 MeowLive2D 项目或启动器配置，请检查环境页中的发行版选择。",
  desktop_database_failed: "WSL PostgreSQL 启动失败，请确认 Docker 已运行且项目数据库配置有效。",
  desktop_server_ready: "桌面主服务已就绪", desktop_server_failed: "桌面主服务启动或运行失败",
  desktop_server_stopped: "桌面主服务已停止", desktop_server_waiting: "桌面主服务尚未就绪", desktop_runtime_running: "桌面执行端正在运行",
  desktop_runtime_failed: "桌面执行端运行失败", desktop_runtime_stopped: "桌面执行端已停止",
};
/** Older native shells expose status only. Observe transitions without retaining error text. */
function observeDesktopLogs(status: DesktopStatus): RuntimeLogListResponse {
  const identity = `${status.config_path}\n${status.server_url}`;
  if (identity !== observedDesktop) {
    observedDesktop = identity; previousServerState = ""; previousRuntimeState = "";
    desktopEvents.length = 0; desktopTruncated = false;
  }
  const diagnostic = status.server.last_error?.includes("Wsl/Service/0x8007274c") ? "desktop_wsl_timeout"
    : status.server.last_error?.includes("PostgreSQL 启动失败") ? "desktop_database_failed"
    : status.server.last_error?.includes("找不到选定 WSL") ? "desktop_wsl_project_missing" : "";
  const serverState = status.server.stopped ? "stopped" : status.server.ready ? "ready" : status.server.last_error ? "failed" : "waiting";
  const runtimeState = status.runtime.running ? "running" : status.runtime.last_error ? "failed" : "stopped";
  for (const [part, state, previous] of [["server", serverState, previousServerState], ["runtime", runtimeState, previousRuntimeState]]) {
    const observed = part === "server" && state === "failed" && diagnostic ? diagnostic : state;
    if (observed === previous) continue;
    const code = part === "server" && state === "failed" && diagnostic ? diagnostic : `desktop_${part}_${state}`;
    desktopEvents.unshift({ id: `desktop-${++eventSequence}`, timestamp: new Date().toISOString(), source: "desktop", category: part === "server" ? "server" : "execution",
      level: state === "failed" ? "error" : state === "stopped" ? "warn" : "info", code, summary: desktopSummaries[code] });
    if (desktopEvents.length > 100) { desktopEvents.length = 100; desktopTruncated = true; }
  }
  previousServerState = serverState === "failed" && diagnostic ? diagnostic : serverState; previousRuntimeState = runtimeState;
  return { entries: [...desktopEvents], storage_available: false, truncated: desktopTruncated };
}

export async function getDesktopStatus(): Promise<DesktopStatus | null> {
  if (!("__TAURI_INTERNALS__" in globalThis)) {
    const value = await browserDesktopStatus();
    if (value === null) return null;
    const status = readDesktopStatus(value);
    return { ...status, runtime_logs: status.runtime_logs ?? observeDesktopLogs(status) };
  }
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

export async function setDesktopServiceEnabled(id: LauncherServiceId, enabled: boolean): Promise<void> {
  await invokeDesktop("desktop_service_set_enabled", { id, enabled });
}

export async function openControlPanel(): Promise<void> {
  if (!("__TAURI_INTERNALS__" in globalThis)) {
    window.open("http://127.0.0.1:1420", "_blank", "noopener,noreferrer");
    return;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_control_panel");
}
