import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { DesktopEnvironmentPanel } from "./DesktopEnvironmentPanel";
import type { EnvironmentClient, EnvironmentSnapshot } from "../../services/desktop/environment";

const snapshot = (overrides: Partial<EnvironmentSnapshot> = {}): EnvironmentSnapshot => ({
  phase: "idle", busy: false, message: "检测完成", logs: [], distros: [{ name: "Ubuntu", version: 2 }],
  selectedDistro: "Ubuntu", backend: { ready: false, engineRoot: "", pythonPath: "", gpu: false, detail: "需要安装" },
  models: [], progress: 0, inferenceRunning: false, ...overrides,
});
function client(state: EnvironmentSnapshot): EnvironmentClient {
  return { status: vi.fn().mockResolvedValue(state), action: vi.fn().mockResolvedValue(state), apply: vi.fn().mockResolvedValue(undefined) };
}
it("allows installation without a ready main service and requires a WSL2 choice", async () => {
  const api = client(snapshot({ distros: [{ name: "旧环境", version: 1 }, { name: "Ubuntu", version: 2 }] }));
  render(<DesktopEnvironmentPanel client={api} />);
  await screen.findByText("需要安装");
  expect(screen.getByRole("option", { name: "旧环境 · WSL1（需要升级）" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "安装或修复训练后端" }));
  expect(api.action).toHaveBeenCalledWith({ action: "install_backend", distro: "Ubuntu" });
});
it("keeps model selection separate from applying and does not enable unsupported models", async () => {
  const api = client(snapshot({ backend: { ready: true, engineRoot: "/engine", pythonPath: "/python", gpu: true, detail: "就绪" },
    models: [{ id: "gpt-sovits-v2", name: "GPT-SoVITS v2", capability: "training_inference", downloaded: true, selected: false },
      { id: "other", name: "未适配模型", capability: "download_only", downloaded: true, selected: false }] }));
  render(<DesktopEnvironmentPanel client={api} />);
  await screen.findByText("GPT-SoVITS v2");
  expect(screen.getByRole("button", { name: "选用 未适配模型" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "选用 GPT-SoVITS v2" }));
  expect(api.action).toHaveBeenCalledWith({ action: "select_model", distro: "Ubuntu", modelId: "gpt-sovits-v2" });
  expect(api.apply).not.toHaveBeenCalled();
});
it("reports installation failure and keeps a retry path", async () => {
  const api = client(snapshot());
  vi.mocked(api.action).mockRejectedValue(new Error("发行版不可用"));
  render(<DesktopEnvironmentPanel client={api} />);
  await screen.findByText("需要安装");
  await userEvent.click(screen.getByRole("button", { name: "安装或修复训练后端" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("发行版不可用");
  await waitFor(() => expect(screen.getByRole("button", { name: "重新检测" })).toBeEnabled());
});
it("shows reboot resume and model progress without a false ready state", async () => {
  const api = client(snapshot({ phase: "reboot_required", message: "重启 Windows 后继续安装", distros: [], selectedDistro: null }));
  render(<DesktopEnvironmentPanel client={api} />);
  expect(await screen.findByText("重启 Windows 后继续安装")).toBeVisible();
  expect(screen.getByRole("button", { name: "连接训练后端" })).toBeDisabled();
});
