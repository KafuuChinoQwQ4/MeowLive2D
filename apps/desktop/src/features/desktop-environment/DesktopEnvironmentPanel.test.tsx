import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { DesktopEnvironmentPanel } from "./DesktopEnvironmentPanel";
import type { EnvironmentClient, EnvironmentSnapshot } from "../../services/desktop/environment";

const snapshot = (overrides: Partial<EnvironmentSnapshot> = {}): EnvironmentSnapshot => ({
  phase: "idle", busy: false, message: "检测完成", logs: [], distros: [{ name: "Ubuntu", version: 2 }],
  selectedDistro: "Ubuntu", backend: { ready: false, engineRoot: "", pythonPath: "", modelRoot: "", gpu: false, detail: "需要安装" },
  models: [], progress: 0, inferenceRunning: false, ...overrides,
});
function client(state: EnvironmentSnapshot): EnvironmentClient {
  return { status: vi.fn().mockResolvedValue(state), action: vi.fn().mockResolvedValue(state), apply: vi.fn().mockResolvedValue(undefined) };
}
it("searches the full catalog by name or id and keeps download-only models unavailable for selection", async () => {
  const api = client(snapshot({ backend: { ready: true, engineRoot: "/engine", pythonPath: "/python", modelRoot: "/models", gpu: true, detail: "就绪" },
    models: [{ id: "gpt-sovits-v2", name: "GPT-SoVITS v2", capability: "training_inference", downloaded: true, selected: false },
      { id: "qwen3-asr-1.7b", name: "Qwen3-ASR 1.7B", capability: "download_only", downloaded: false, selected: false }] }));
  render(<DesktopEnvironmentPanel client={api} />);
  const search = await screen.findByRole("searchbox", { name: "搜索声音模型" });
  await userEvent.type(search, "QWEN3-ASR-1.7b");
  expect(screen.queryByRole("heading", { name: "GPT-SoVITS v2" })).not.toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Qwen3-ASR 1.7B" })).toBeVisible();
  expect(screen.getByRole("button", { name: "选用 Qwen3-ASR 1.7B" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "下载 Qwen3-ASR 1.7B" }));
  expect(api.action).toHaveBeenCalledWith({ action: "download_model", distro: "Ubuntu", modelId: "qwen3-asr-1.7b" });
  await userEvent.clear(search);
  await userEvent.type(search, "不存在的模型");
  expect(screen.getByText("没有匹配的模型。")).toBeVisible();
});
it("allows installation without a ready main service and requires a WSL2 choice", async () => {
  const api = client(snapshot({ distros: [{ name: "旧环境", version: 1 }, { name: "Ubuntu", version: 2 }] }));
  render(<DesktopEnvironmentPanel client={api} />);
  await screen.findByText("需要安装");
  await userEvent.click(screen.getByText("环境配置与修复"));
  expect(screen.getByRole("option", { name: "旧环境 · WSL1（需要升级）" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "安装或修复训练后端" }));
  expect(api.action).toHaveBeenCalledWith({ action: "install_backend", distro: "Ubuntu" });
});
it("keeps model selection separate from applying and does not enable unsupported models", async () => {
  const api = client(snapshot({ backend: { ready: true, engineRoot: "/engine", pythonPath: "/python", modelRoot: "/models", gpu: true, detail: "就绪" },
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
  await userEvent.click(screen.getByText("环境配置与修复"));
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

it("imports an existing config and exposes the detected paths before connecting", async () => {
  const state = snapshot();
  const api = client(state);
  vi.mocked(api.action).mockImplementation(async request => {
    if (request.configPath !== "/home/me/custom config.toml" || request.distro !== "Ubuntu") throw new Error("错误的配置路径");
    return snapshot({ backend: { ready: true, engineRoot: "/custom/GPT-SoVITS", pythonPath: "/custom/python", modelRoot: "/custom/models", gpu: true, detail: "现有环境可用" },
      models: [{ id: "gpt-sovits-v2", name: "GPT-SoVITS v2", capability: "training_inference", downloaded: true, selected: false }] });
  });
  render(<DesktopEnvironmentPanel client={api} />);
  await screen.findByText("需要安装");
  await userEvent.click(screen.getByText("环境配置与修复"));
  await userEvent.click(screen.getByText("使用已有训练环境"));
  await userEvent.type(screen.getByLabelText("WSL 配置文件路径"), "/home/me/custom config.toml");
  await userEvent.click(screen.getByRole("button", { name: "重新检测" }));
  expect(await screen.findByText("现有环境可用")).toBeVisible();
  expect(screen.getByText(/Python：/)).toHaveTextContent("/custom/python");
  expect(screen.getByRole("button", { name: "选用 GPT-SoVITS v2" })).toBeEnabled();
  expect(screen.getByRole("button", { name: "连接训练后端" })).toBeDisabled();
});

it("explains the blocked connection and lets the user select the detected model beside it", async () => {
  const state = snapshot({ backend: { ready: true, engineRoot: "/engine", pythonPath: "/python", modelRoot: "/models", gpu: true, detail: "现有环境可用" },
    models: [{ id: "gpt-sovits-v2", name: "GPT-SoVITS v2", capability: "training_inference", downloaded: true, selected: false }] });
  const api = client(state);
  vi.mocked(api.action).mockImplementation(async request => {
    if (request.action !== "select_model" || request.modelId !== "gpt-sovits-v2") throw new Error("错误的模型选择");
    return { ...state, models: [{ ...state.models[0], selected: true }] };
  });
  render(<DesktopEnvironmentPanel client={api} />);
  const connect = await screen.findByRole("button", { name: "连接训练后端" });
  expect(connect).toBeDisabled();
  expect(connect).toHaveAccessibleDescription(/尚未选用/);
  await userEvent.click(screen.getByRole("button", { name: "选用已检测模型" }));
  await waitFor(() => expect(connect).toBeEnabled());
  expect(api.apply).not.toHaveBeenCalled();
  await userEvent.click(connect);
  expect(await screen.findByText(/训练后端已连接/)).toBeVisible();
});


it("keeps diagnostics collapsed and shows indeterminate detection without a fake percentage", async () => {
  render(<DesktopEnvironmentPanel client={client(snapshot({ busy: true, phase: "detecting", message: "正在检测环境", progress: 0 }))} />);
  const progress = await screen.findByRole("progressbar", { name: "环境任务进度" });
  expect(progress).not.toHaveAttribute("aria-valuenow");
  expect(screen.getByText("环境配置与修复").closest("details")).not.toHaveAttribute("open");
  expect(screen.getByText("环境检测与安装日志").closest("details")).not.toHaveAttribute("open");
  expect(screen.getByRole("button", { name: "取消当前任务" })).toBeEnabled();
});

it("reports actual download progress", async () => {
  render(<DesktopEnvironmentPanel client={client(snapshot({ busy: true, phase: "downloading", progress: 42 }))} />);
  expect(await screen.findByRole("progressbar", { name: "环境任务进度" })).toHaveAttribute("aria-valuenow", "42");
});

it("shows native Linux detection and the shared model browser without WSL installation", async () => {
  const api = client(snapshot({ distros: [], selectedDistro: null, backend: { ready: false, engineRoot: "", pythonPath: "", modelRoot: "", gpu: false, detail: "尚未安装本机语音后端" } }));
  render(<DesktopEnvironmentPanel platform="linux" client={api} />);
  expect(await screen.findByText("尚未安装本机语音后端")).toBeVisible();

  expect(screen.queryByText("训练环境")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "连接训练后端" })).not.toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "选择声音模型" })).toBeVisible();
  expect(api.action).not.toHaveBeenCalled();
});

it("paginates the Linux catalog and resets the page when a category tag changes", async () => {
  const models = Array.from({ length: 7 }, (_, i) => ({ id: `model-${i}`, name: `模型 ${i}`, capability: "training_inference", downloaded: false, selected: false }));
  models.push({ id: "whisper-small", name: "Whisper Small", capability: "transcription", downloaded: false, selected: false });
  render(<DesktopEnvironmentPanel platform="linux" client={client(snapshot({ models }))} />);
  await screen.findByRole("heading", { name: "模型 0" });
  expect(screen.getAllByRole("article")).toHaveLength(4);
  await userEvent.click(screen.getByRole("button", { name: "下一页" }));
  expect(screen.getByRole("heading", { name: "模型 4" })).toBeVisible();
  await userEvent.click(screen.getByRole("button", { name: "语音转文字" }));
  expect(screen.getAllByRole("article")).toHaveLength(1);
  expect(screen.getByRole("heading", { name: "Whisper Small" })).toBeVisible();
  expect(screen.getByText("共 1 个模型 · 第 1 / 1 页")).toBeVisible();
});
