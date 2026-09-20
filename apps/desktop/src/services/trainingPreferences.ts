import type { TrainingPerformance } from "@meowlive/contracts";
import { defaultTrainingPerformance } from "./server/training";

export type TrainingPerformancePreset = "low" | "balanced" | "high" | "custom";
export interface TrainingPreferences {
  epochs: number;
  performancePreset: TrainingPerformancePreset;
  performance: TrainingPerformance;
  textMode: boolean;
  auditionText: string;
}

function defaults(): TrainingPreferences {
  return {
    epochs: 1,
    performancePreset: "low",
    performance: { ...defaultTrainingPerformance },
    textMode: false,
    auditionText: "你好，欢迎来到直播间，希望你今天过得愉快。",
  };
}

const object = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const integer = (value: unknown, min: number, max: number): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= min && value <= max;

function validate(value: unknown): TrainingPreferences | undefined {
  if (!object(value) || !integer(value.epochs, 1, 20) || typeof value.performancePreset !== "string"
    || !["low", "balanced", "high", "custom"].includes(value.performancePreset) || typeof value.textMode !== "boolean"
    || typeof value.auditionText !== "string" || value.auditionText.length > 500
    || /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/u.test(value.auditionText)) return;
  const performance = value.performance;
  if (!object(performance) || !integer(performance.batch_size, 1, 16) || !integer(performance.data_workers, 0, 8)
    || !integer(performance.cpu_threads, 1, 16) || !integer(performance.gpu_index, 0, 15) || typeof performance.low_memory !== "boolean") return;
  return {
    epochs: value.epochs,
    performancePreset: value.performancePreset as TrainingPerformancePreset,
    performance: {
      batch_size: performance.batch_size,
      data_workers: performance.data_workers,
      cpu_threads: performance.cpu_threads,
      gpu_index: performance.gpu_index,
      low_memory: performance.low_memory,
    },
    textMode: value.textMode,
    auditionText: value.auditionText,
  };
}

function storageKey(serverUrl?: string): string | undefined {
  if (!serverUrl) return;
  const url = new URL(serverUrl);
  if (url.protocol !== "http:" && url.protocol !== "https:") return;
  url.username = ""; url.password = ""; url.search = ""; url.hash = "";
  return `meowlive.training-preferences.v1:${url.href.replace(/\/+$/u, "")}`;
}

export function loadTrainingPreferences(serverUrl?: string): TrainingPreferences {
  try {
    const key = storageKey(serverUrl);
    const raw = key ? globalThis.localStorage.getItem(key) : null;
    if (raw && raw.length <= 8192) return validate(JSON.parse(raw)) ?? defaults();
  } catch {
    // Local preferences must not prevent training when storage is blocked or corrupt.
  }
  return defaults();
}

export function saveTrainingPreferences(serverUrl: string | undefined, preferences: TrainingPreferences): void {
  try {
    const key = storageKey(serverUrl);
    const valid = validate(preferences);
    if (key && valid) globalThis.localStorage.setItem(key, JSON.stringify(valid));
  } catch {
    // Keep the editable in-memory values when browser storage is unavailable or full.
  }
}
