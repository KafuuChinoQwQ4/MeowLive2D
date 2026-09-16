import type { ModelLibrarySnapshot } from "@meowlive/contracts";

export interface ModelLibraryClient {
  getStatus(signal?: AbortSignal): Promise<ModelLibrarySnapshot>;
  scan(token: string, signal?: AbortSignal): Promise<ModelLibrarySnapshot>;
  select(id: string, token: string, signal?: AbortSignal): Promise<ModelLibrarySnapshot>;
  download(id: string, token: string, signal?: AbortSignal): Promise<ModelLibrarySnapshot>;
  cancel(id: string, token: string, signal?: AbortSignal): Promise<ModelLibrarySnapshot>;
}
class ModelLibraryError extends Error {}
const string = (value: unknown): value is string => typeof value === "string" && value.length <= 8192;
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const strings = (value: Record<string, unknown>, keys: string[]) => keys.every(key => string(value[key]));
const list = (value: unknown, validator: (item: unknown) => boolean) => Array.isArray(value) && value.length <= 256 && value.every(validator);
const https = (value: unknown) => { if (!string(value)) return false; try { const url = new URL(value); return url.protocol === "https:" && !url.username && !url.password; } catch { return false; } };
const bytes = (value: unknown) => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
function readSnapshot(value: unknown): ModelLibrarySnapshot {
  if (!object(value) || value.schema_version !== 1 || !(value.selected_id === null || string(value.selected_id))
    || !object(value.environment) || !strings(value.environment, ["kind", "release", "distro", "message"])
    || !["wsl2", "wsl1", "linux", "unknown"].includes(String(value.environment.kind)) || typeof value.environment.ready !== "boolean"
    || !object(value.runtime) || !strings(value.runtime, ["engine_root", "python_path", "message"]) || typeof value.runtime.ready !== "boolean"
    || !list(value.scan_roots, string)
    || !list(value.installed, item => object(item) && strings(item, ["id", "model_id", "name", "path", "message"]) && typeof item.ready === "boolean" && typeof item.selected === "boolean")
    || !list(value.catalog, item => object(item) && strings(item, ["id", "name", "languages", "description", "license", "note"]) && https(item.homepage) && https(item.source_url) && ["ready", "download_only"].includes(String(item.compatibility)))
    || !list(value.downloads, item => object(item) && strings(item, ["id", "model_id", "message", "path"]) && ["queued", "downloading", "completed", "failed", "cancelled"].includes(String(item.state)) && bytes(item.downloaded_bytes) && bytes(item.total_bytes))) {
    throw new ModelLibraryError("环境与模型返回了无效状态，请刷新控制面板。");
  }
  return value as unknown as ModelLibrarySnapshot;
}
export function createModelLibraryClient(options: { fetcher?: typeof fetch; timeoutMs?: number } = {}): ModelLibraryClient {
  const fetcher = options.fetcher ?? ((...args: Parameters<typeof fetch>) => globalThis.fetch(...args));
  async function request(path: string, body?: object, token?: string, signal?: AbortSignal): Promise<ModelLibrarySnapshot> {
    signal?.throwIfAborted();
    const controller = new AbortController();
    const cancel = () => controller.abort();
    signal?.addEventListener("abort", cancel, { once: true });
    let timedOut = false;
    const timer = setTimeout(() => { timedOut = true; controller.abort(); }, options.timeoutMs ?? 15000);
    const aborted = new Promise<never>((_resolve, reject) => controller.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true }));
    try {
      const response = await Promise.race([fetcher(`/api/launcher/models${path}`, { method: body ? "POST" : "GET", ...(body ? { headers: { "Content-Type": "application/json", "X-MeowLive-Launcher-Token": token! }, body: JSON.stringify(body) } : {}), signal: controller.signal, cache: "no-store" }), aborted]);
      const raw = await Promise.race([response.text(), aborted]);
      if (raw.length > 1048576) throw new ModelLibraryError("模型列表过大，请减少扫描目录中的文件后重试。");
      let data: unknown;
      try { data = JSON.parse(raw); } catch { throw new ModelLibraryError("模型管理不可用，请使用 ./launchers/start.sh 启动控制面板。"); }
      if (!response.ok) throw new ModelLibraryError(object(data) && string(data.message) ? data.message : "模型操作失败，请刷新状态后重试。");
      return readSnapshot(data);
    } catch (error) {
      if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
      if (timedOut) throw new ModelLibraryError("模型管理请求超时，请刷新状态确认结果。");
      if (error instanceof ModelLibraryError) throw error;
      throw new ModelLibraryError("无法连接模型管理，请保持 ./launchers/start.sh 的终端打开。");
    } finally { clearTimeout(timer); signal?.removeEventListener("abort", cancel); }
  }
  return { getStatus: signal => request("", undefined, undefined, signal), scan: (token, signal) => request("/scan", {}, token, signal),
    select: (id, token, signal) => request("/select", { id }, token, signal), download: (id, token, signal) => request("/download", { id }, token, signal), cancel: (id, token, signal) => request("/cancel", { id }, token, signal) };
}
