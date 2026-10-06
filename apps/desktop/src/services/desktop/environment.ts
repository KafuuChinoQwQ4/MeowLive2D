/** Windows 环境 IPC 边界；功能组件不直接访问 Tauri。 */
import type { EnvironmentSnapshot } from "@meowlive/contracts";
import { invokeDesktop } from "./transport";
export type { EnvironmentSnapshot } from "@meowlive/contracts";
export interface EnvironmentRequest {
  action: "detect" | "install_wsl" | "install_backend" | "download_model" | "select_model" | "start_inference" | "stop_inference" | "cancel";
  distro?: string;
  modelId?: string;
  configPath?: string;
}
export interface EnvironmentClient {
  status(): Promise<EnvironmentSnapshot>;
  action(request: EnvironmentRequest): Promise<EnvironmentSnapshot>;
  apply(): Promise<void>;
}
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === "object" && !Array.isArray(value);
const string = (value: unknown): value is string => typeof value === "string" && value.length <= 8192;
export function readEnvironmentSnapshot(value: unknown): EnvironmentSnapshot {
  if (!object(value) || !["idle", "detecting", "installing", "downloading", "running", "failed", "reboot_required"].includes(String(value.phase))
    || typeof value.busy !== "boolean" || !string(value.message) || !Array.isArray(value.logs) || value.logs.length > 500 || !value.logs.every(string)
    || !Array.isArray(value.distros) || value.distros.length > 128 || !value.distros.every(item => object(item) && string(item.name) && [1, 2].includes(Number(item.version)))
    || !(value.selectedDistro === null || string(value.selectedDistro)) || !object(value.backend) || typeof value.backend.ready !== "boolean"
    || typeof value.backend.gpu !== "boolean" || ![value.backend.engineRoot, value.backend.pythonPath, value.backend.modelRoot, value.backend.detail].every(string)
    || !Array.isArray(value.models) || value.models.length > 128 || !value.models.every(item => object(item) && [item.id, item.name, item.capability].every(string) && typeof item.downloaded === "boolean" && typeof item.selected === "boolean")
    || typeof value.progress !== "number" || !Number.isFinite(value.progress) || value.progress < 0 || value.progress > 100
    || typeof value.inferenceRunning !== "boolean") throw new Error("环境状态无效，请重新检测。");
  return value as unknown as EnvironmentSnapshot;
}
export function createEnvironmentClient(): EnvironmentClient {
  async function call(command: string, args?: Record<string, unknown>): Promise<unknown> {
    return invokeDesktop(command, args);
  }
  return {
    status: async () => readEnvironmentSnapshot(await call("environment_status")),
    action: async request => readEnvironmentSnapshot(await call("environment_action", { request })),
    apply: async () => { await call("environment_apply"); },
  };
}
