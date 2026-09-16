/**
 * Tauri 桌面命令客户端边界。
 * 浏览器开发模式与桌面可用能力在此区分；各 feature 不直接导入 Tauri SDK。
 */
export interface DesktopStatus {
  config_path: string;
  server_url: string;
  runtime: { running: boolean; simulation: boolean; last_error: string | null };
}

export function readDesktopStatus(value: unknown): DesktopStatus {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid desktop status");
  const status = value as Record<string, unknown>;
  const runtime = status.runtime as Record<string, unknown> | undefined;
  if (typeof status.config_path !== "string" || typeof status.server_url !== "string" || !validOrigin(status.server_url) || !runtime || typeof runtime.running !== "boolean" || typeof runtime.simulation !== "boolean" || !(runtime.last_error === null || typeof runtime.last_error === "string")) throw new Error("Invalid desktop status");
  return { config_path: status.config_path, server_url: status.server_url, runtime: runtime as DesktopStatus["runtime"] };
}

function validOrigin(value: string): boolean {
  try {
    const url = new URL(value);
    return value.length <= 4096 && !/\s/u.test(value) && ["http:", "https:"].includes(url.protocol)
      && !url.username && !url.password && !url.search && !url.hash && url.pathname === "/"
      && /^https?:\/\/[^/?#]+$/u.test(value);
  } catch { return false; }
}

export async function getDesktopStatus(): Promise<DesktopStatus | null> {
  if (!("__TAURI_INTERNALS__" in globalThis)) return null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const value = await Promise.race([
      import("@tauri-apps/api/core").then(({ invoke }) => invoke("desktop_status")),
      new Promise<never>((_resolve, reject) => { timer = setTimeout(() => reject(new Error("桌面状态读取超时")), 5_000); }),
    ]);
    return readDesktopStatus(value);
  } finally { clearTimeout(timer); }
}
