/** Shared native IPC and same-origin browser bridge. Never exposes window IPC to a webpage. */
let browserToken: string | null = null;
let browserConnected = false;

async function request(path: string, init?: RequestInit, timeout = 5_000): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeout);
  try { return await fetch(path, { ...init, signal: controller.signal, cache: "no-store", credentials: "same-origin" }); }
  finally { clearTimeout(timer); }
}

export async function browserDesktopStatus(): Promise<unknown | null> {
  try {
    const response = await request("/api/desktop/status");
    if (response.status === 404) { browserToken = null; browserConnected = false; return null; }
    if (!response.ok) {
      browserConnected = true;
      throw new Error("无法读取 App 控制接口，请保持 App 运行。");
    }
    const body: unknown = await response.json();
    if (!body || typeof body !== "object" || !("token" in body) || !("status" in body)) {
      if (browserConnected) throw new Error("App 控制接口返回了无效状态。");
      return null;
    }
    const token = body.token;
    if (typeof token !== "string" || !/^[a-f0-9]{64}$/.test(token)) throw new Error("App 控制接口验证失败。");
    browserToken = token;
    browserConnected = true;
    return body.status;
  } catch (failure) {
    if (browserConnected) throw failure;
    browserToken = null;
    return null;
  }
}

export async function invokeDesktop<T = unknown>(command: string, args?: Record<string, unknown>): Promise<T> {
  if ("__TAURI_INTERNALS__" in globalThis) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }
  if (!browserToken && !(await browserDesktopStatus())) throw new Error("未连接 App，请保持 App 运行后刷新页面。");
  const response = await request("/api/desktop/command", { method: "POST", headers: {
    "Content-Type": "application/json", "X-MeowLive-Desktop-Token": browserToken!,
  }, body: JSON.stringify({ command, ...(args ? { args } : {}) }) }, 120_000);
  const body = await response.json() as { result?: T; error?: string };
  if (!response.ok) throw new Error(body.error || "App 控制操作失败，请刷新状态。");
  return body.result as T;
}
