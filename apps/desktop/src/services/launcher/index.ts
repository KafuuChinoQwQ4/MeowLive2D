import type { LauncherService, LauncherServiceId, LauncherSnapshot } from "@meowlive/contracts";

export interface LauncherClient {
  getStatus(signal?: AbortSignal): Promise<LauncherSnapshot>;
  setEnabled(id: LauncherServiceId, enabled: boolean, token: string, signal?: AbortSignal): Promise<LauncherSnapshot>;
}

class LauncherError extends Error {}
const states = new Set(["stopped", "starting", "running", "stopping", "failed", "external"]);
function text(value: unknown): value is string { return typeof value === "string" && value.length <= 4096; }
function validService(value: unknown): value is LauncherService {
  if (!value || typeof value !== "object") return false;
  const service = value as Partial<LauncherService>;
  return (service.id === "server" || service.id === "tts" || service.id === "windows") && states.has(service.state ?? "")
    && typeof service.managed === "boolean" && text(service.message) && text(service.url) && text(service.log_path)
    && typeof service.can_start === "boolean" && typeof service.can_stop === "boolean"
    && (!service.can_stop || service.managed) && (!service.can_start || (!service.managed && ["stopped", "failed"].includes(service.state ?? "")));
}

function readSnapshot(value: unknown): LauncherSnapshot {
  const data = value as Partial<LauncherSnapshot> | null;
  const setup = data?.setup;
  if (!data || data.schema_version !== 1 || !/^[a-f0-9]{64}$/.test(data.session_token ?? "")
    || !Array.isArray(data.services) || data.services.length !== 3 || !data.services.every(validService)
    || new Set(data.services.map(service => service.id)).size !== 3
    || !setup || !text(setup.configuration_path) || !text(setup.server_config) || !text(setup.windows_client_path)
    || typeof setup.llm_configured !== "boolean" || !text(setup.llm_message)) {
    throw new LauncherError("启动管理返回了无效状态，请确认使用 ./launchers/start.sh 打开面板。");
  }
  return data as LauncherSnapshot;
}

export function createLauncherClient(options: { fetcher?: typeof fetch; timeoutMs?: number } = {}): LauncherClient {
  const fetcher = options.fetcher ?? ((...args: Parameters<typeof fetch>) => globalThis.fetch(...args));
  async function request(path: string, init: RequestInit, signal?: AbortSignal): Promise<LauncherSnapshot> {
    signal?.throwIfAborted();
    const controller = new AbortController();
    const cancel = () => controller.abort();
    signal?.addEventListener("abort", cancel, { once: true });
    let timedOut = false;
    const timer = setTimeout(() => { timedOut = true; controller.abort(); }, options.timeoutMs ?? 5000);
    const aborted = new Promise<never>((_resolve, reject) => controller.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true }));
    try {
      const response = await Promise.race([fetcher(`/api/launcher/${path}`, { ...init, signal: controller.signal, cache: "no-store" }), aborted]);
      const body = await Promise.race([response.text(), aborted]);
      if (body.length > 65536) throw new LauncherError("启动管理返回的状态过大。");
      let value: unknown;
      try { value = JSON.parse(body); } catch { throw new LauncherError("启动管理不可用，请使用 ./launchers/start.sh 打开控制面板。"); }
      if (!response.ok) {
        const message = (value as { message?: unknown } | null)?.message;
        throw new LauncherError(text(message) ? message : "启动管理操作失败，请刷新状态后重试。");
      }
      return readSnapshot(value);
    } catch (error) {
      if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
      if (timedOut) throw new LauncherError("启动管理请求超时，请刷新状态确认服务是否已启动。");
      if (error instanceof LauncherError) throw error;
      throw new LauncherError("无法连接启动管理，请检查运行 ./launchers/start.sh 的终端是否仍然打开。");
    } finally { clearTimeout(timer); signal?.removeEventListener("abort", cancel); }
  }
  return {
    getStatus: signal => request("status", { method: "GET" }, signal),
    setEnabled: (id, enabled, token, signal) => request(`services/${id}`, { method: "POST",
      headers: { "Content-Type": "application/json", "X-MeowLive-Launcher-Token": token }, body: JSON.stringify({ enabled }) }, signal),
  };
}
