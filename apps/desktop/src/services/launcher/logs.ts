import { readRuntimeLogs, requestRuntimeLogs, runtimeLogParams, type RuntimeLogClient } from "../server/logs";

export function createLauncherLogClient(options: { fetcher?: typeof fetch; timeoutMs?: number } = {}): Pick<RuntimeLogClient, "list"> {
  const fetcher = options.fetcher ?? ((...args: Parameters<typeof fetch>) => globalThis.fetch(...args));
  return { async list(query = {}, signal) {
    return readRuntimeLogs(await requestRuntimeLogs(fetcher, `/api/launcher/logs${runtimeLogParams(query)}`, { method: "GET" }, signal, options.timeoutMs));
  } };
}
