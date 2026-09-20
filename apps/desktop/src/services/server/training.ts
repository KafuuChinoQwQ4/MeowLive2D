import type { TrainingCreateRequest, TrainingPerformance, TrainingModelRuntimeSnapshot, TrainingJob, TrainingSnapshot, TrainingTranscription, RuntimePresetSnapshot } from "@meowlive/contracts";
import { readServerError, ServerRequestError } from "./responses";
import { createAuthenticatedFetch } from "./auth";

export interface TrainingClient {
  modelStatus(signal?: AbortSignal): Promise<TrainingModelRuntimeSnapshot>;
  setModelsEnabled(enabled: boolean, signal?: AbortSignal): Promise<TrainingModelRuntimeSnapshot>;
  snapshot(signal?: AbortSignal): Promise<TrainingSnapshot>;
  create(metadata: TrainingCreateRequest, audio: File[], signal?: AbortSignal): Promise<TrainingJob>;
  transcribe(audio: File, language: string, signal?: AbortSignal): Promise<TrainingTranscription>;
  cancel(id: string, signal?: AbortSignal): Promise<TrainingSnapshot>;
  delete(id: string, signal?: AbortSignal): Promise<TrainingSnapshot>;
  save(id: string, signal?: AbortSignal): Promise<TrainingSnapshot>;
  activate(id: string, signal?: AbortSignal): Promise<TrainingSnapshot>;
  audition(version: string, text: string, signal?: AbortSignal): Promise<Blob>;
  preset(signal?: AbortSignal): Promise<RuntimePresetSnapshot>;
  measure(voice: string, text: string, signal?: AbortSignal): Promise<RuntimePresetSnapshot>;
}
const object = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const text = (v: unknown, max = 128): v is string => typeof v === "string" && v.length > 0 && Array.from(v).length <= max && !/[\u0000-\u001f\u007f]/u.test(v);
const speechText = (v: unknown): v is string => typeof v === "string" && v.trim().length > 0 && Array.from(v).length <= 500 && !/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/u.test(v);
const number = (v: unknown, max = Number.MAX_SAFE_INTEGER): v is number => Number.isSafeInteger(v) && (v as number) >= 0 && (v as number) <= max;
const id = (v: unknown): v is string => typeof v === "string" && /^[a-zA-Z0-9_-]{1,64}$/u.test(v);
const invalid = () => new ServerRequestError("invalid_response", "训练服务返回的数据无效");
export const defaultTrainingPerformance: TrainingPerformance = { batch_size: 1, data_workers: 1, cpu_threads: 2, gpu_index: 0, low_memory: true };
function readPerformance(v: unknown): TrainingPerformance {
  if (v === undefined) return { ...defaultTrainingPerformance };
  if (!object(v) || !number(v.batch_size, 16) || v.batch_size < 1 || !number(v.data_workers, 8)
    || !number(v.cpu_threads, 16) || v.cpu_threads < 1 || !number(v.gpu_index, 15) || typeof v.low_memory !== "boolean") throw invalid();
  return v as unknown as TrainingPerformance;
}
function readModelStatus(v: unknown): TrainingModelRuntimeSnapshot {
  if (!object(v) || typeof v.supported !== "boolean" || typeof v.state !== "string" || !text(v.message, 500)
    || !(v.supported ? ["loaded", "unloaded", "loading", "unloading", "failed", "unavailable"] : ["unsupported"]).includes(v.state)) throw invalid();
  return v as unknown as TrainingModelRuntimeSnapshot;
}
function readTranscription(v: unknown, language: string): TrainingTranscription {
  if (!object(v) || !text(v.text, 500) || !v.text.trim() || /[|\u0080-\u009f]/u.test(v.text)
    || typeof v.language !== "string" || v.language !== language || !["zh", "en", "ja", "ko", "yue"].includes(v.language)) throw invalid();
  return { text: v.text, language: v.language };
}
export function readJob(v: unknown): TrainingJob {
  if (!object(v) || !id(v.id) || !id(v.voice_id) || !text(v.name, 80) || !text(v.message, 200) || !number(v.progress, 100)
    || !number(v.clip_count, 64) || v.clip_count === 0 || !number(v.created_at_ms) || !number(v.updated_at_ms)
    || !(v.version_id === null || id(v.version_id)) || typeof v.status !== "string" || !["queued", "preparing", "training", "validating", "completed", "failed", "cancelling", "cancelled", "interrupted"].includes(v.status)) throw invalid();
  return { ...v, performance: readPerformance(v.performance) } as TrainingJob;
}
export function readTraining(v: unknown): TrainingSnapshot {
  if (!object(v) || typeof v.enabled !== "boolean" || typeof v.busy !== "boolean" || !Array.isArray(v.jobs) || v.jobs.length > 64 || !Array.isArray(v.versions) || v.versions.length > 64) throw invalid();
  const jobs = v.jobs.map(readJob);
  if (new Set(jobs.map(j => j.id)).size !== jobs.length) throw invalid();
  const ids = new Set<string>();
  for (const version of v.versions) {
    if (!object(version) || !id(version.id) || ids.has(version.id) || !id(version.job_id) || !id(version.voice_id) || !text(version.name, 80)
      || version.engine !== "gpt-sovits" || version.model_version !== "v2" || typeof version.auditioned !== "boolean" || typeof version.active !== "boolean"
      || typeof version.saved !== "boolean" || (version.saved && !version.auditioned) || (version.active && !version.saved)
      || typeof version.available !== "boolean" || !number(version.created_at_ms)
      || !jobs.some(j => j.id === version.job_id && j.voice_id === version.voice_id && j.status === "completed" && j.version_id === version.id)) throw invalid();
    ids.add(version.id);
  }
  return { ...v, jobs } as TrainingSnapshot;
}
export function readPreset(v: unknown): RuntimePresetSnapshot {
  if (!object(v) || typeof v.mode !== "string" || !["cloud", "local"].includes(v.mode) || typeof v.model !== "string" || v.model.length > 128
    || typeof v.local_only !== "boolean" || typeof v.verified !== "boolean" || !number(v.max_tokens, 4096) || !number(v.timeout_seconds, 120) || !text(v.message, 500)) throw invalid();
  const m = v.measurement;
  if (m !== null && (!object(m) || !number(m.measured_at_ms) || !number(m.llm_ms) || !number(m.tts_ms) || !number(m.total_ms)
    || !text(m.gpu_name) || !number(m.gpu_total_mib) || !number(m.gpu_peak_used_mib) || m.gpu_peak_used_mib > m.gpu_total_mib || !id(m.voice_id) || !speechText(m.reply) || typeof m.passed !== "boolean")) throw invalid();
  if (v.verified && (v.mode !== "local" || !v.local_only || !object(m) || m.passed !== true)) throw invalid();
  return v as unknown as RuntimePresetSnapshot;
}
export function createTrainingClient(options: { baseUrl?: string; fetcher?: typeof fetch; timeoutMs?: number } = {}): TrainingClient {
  const base = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/u, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(base);
  async function request(path: string, init: RequestInit, signal?: AbortSignal, audio = false): Promise<unknown> {
    signal?.throwIfAborted();
    const ctrl = new AbortController();
    const abort = () => ctrl.abort();
    signal?.addEventListener("abort", abort, { once: true });
    let timeout = false;
    const timer = setTimeout(() => { timeout = true; ctrl.abort(); }, options.timeoutMs ?? (init.method === "GET" ? 8000 : 430000));
    const cancelled = new Promise<never>((_, reject) => ctrl.signal.addEventListener("abort", () => reject(new DOMException("请求已取消", "AbortError")), { once: true }));
    try {
      const response = await Promise.race([fetcher(base + path, { ...init, signal: ctrl.signal }), cancelled]);
      if (audio && response.ok) {
        if (!response.headers.get("content-type")?.startsWith("audio/wav")) throw invalid();
        const blob = await Promise.race([response.blob(), cancelled]);
        if (blob.size < 44 || blob.size > 8388652) throw invalid();
        return blob;
      }
      const value: unknown = await Promise.race([response.json(), cancelled]);
      if (!response.ok) {
        const error = readServerError(value);
        throw new ServerRequestError(error?.code ?? "http_error", error?.message ?? `主服务请求失败（HTTP ${response.status}）`, response.status);
      }
      return value;
    } catch (error) {
      if (signal?.aborted) throw new DOMException("请求已取消", "AbortError");
      if (timeout) throw new ServerRequestError("request_timeout", path === "/api/training/transcribe" ? "请求超时，请重试提取文本或手工填写" : path === "/api/training/models" ? "模型状态请求超时，请稍后查看当前状态" : "请求超时；已接收的训练会继续，可在任务列表查看或取消");
      if (error instanceof ServerRequestError) throw error;
      throw new ServerRequestError("connection_failed", "无法连接训练服务，请检查主服务状态");
    } finally { clearTimeout(timer); signal?.removeEventListener("abort", abort); }
  }
  const post = (value: unknown): RequestInit => ({ method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(value) });
  return {
    modelStatus: async signal => readModelStatus(await request("/api/training/models", { method: "GET" }, signal)),
    setModelsEnabled: async (enabled, signal) => readModelStatus(await request("/api/training/models", post({ enabled }), signal)),
    snapshot: async signal => readTraining(await request("/api/training", { method: "GET" }, signal)),
    create: async (metadata, files, signal) => {
      const data = new FormData(); data.append("metadata", JSON.stringify(metadata)); files.forEach(f => data.append("audio", f));
      return readJob(await request("/api/training/jobs", { method: "POST", body: data }, signal));
    },
    transcribe: async (file, language, signal) => {
      const data = new FormData(); data.append("metadata", JSON.stringify({ language })); data.append("audio", file);
      return readTranscription(await request("/api/training/transcribe", { method: "POST", body: data }, signal), language);
    },
    cancel: async (id, signal) => readTraining(await request("/api/training/cancel", post({ id }), signal)),
    delete: async (id, signal) => readTraining(await request("/api/training/delete", post({ id }), signal)),
    save: async (id, signal) => readTraining(await request("/api/training/save", post({ id }), signal)),
    activate: async (id, signal) => readTraining(await request("/api/training/activate", post({ id }), signal)),
    audition: async (version_id, text, signal) => await request("/api/training/audition", post({ version_id, text }), signal, true) as Blob,
    preset: async signal => readPreset(await request("/api/runtime/preset", { method: "GET" }, signal)),
    measure: async (voice_id, text, signal) => readPreset(await request("/api/runtime/measure", post({ voice_id, text }), signal)),
  };
}
