import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { agentStatus } from "../test/agent-fixtures";
import { liveSettingsSnapshot, liveSnapshot } from "../test/live-fixtures";
import { resourceSnapshot } from "../test/resource-fixtures";
import { jsonResponse, serverStatus } from "../test/server-fixtures";

vi.mock("../services/desktop", () => ({ getDesktopStatus: vi.fn(), setDesktopServiceEnabled: vi.fn() }));
import { getDesktopStatus, setDesktopServiceEnabled } from "../services/desktop";
import { App } from "./App";

const llmSnapshot = { settings: { provider: "custom", api_format: "openai_responses", base_url: "", model: "", mode: "cloud", timeout_seconds: 30, max_tokens: 1024, json_mode: true, reasoning_effort: "default" }, key_configured: false, restart_required: false, active_model: "", storage_available: true, profiles: [], selected_profile_id: null };

afterEach(() => { vi.unstubAllGlobals(); vi.resetAllMocks(); window.history.replaceState(null, "", "/"); });

it("waits for desktop configuration and uses its address in every panel", async () => {
  let finish!: (value: Awaited<ReturnType<typeof getDesktopStatus>>) => void;
  vi.mocked(getDesktopStatus).mockReturnValue(new Promise((resolve) => { finish = resolve; }));
  const fetcher = vi.fn<typeof fetch>(async (url) => {
    const path = new URL(String(url)).pathname;
    if (path === "/api/admin/session") return jsonResponse({ enabled: false, authenticated: false });
    if (path === "/api/agent") return jsonResponse(agentStatus());
    if (path === "/api/agent/personas") return jsonResponse({
      profiles: [{ id: "persona-1", name: "配置1", persona: agentStatus().settings.persona }],
      selected_profile_id: "persona-1", storage_available: true,
    });
    if (path === "/api/agent/scheduler") return jsonResponse({ ready: false, phase: "waiting", block_reason: "no_eligible_events", message: "正在等待新的可回应事件", remaining_ms: null, pending_events: 0, deciding_events: 0, active_speeches: 0, updated_at_ms: 1 });
    if (path === "/api/agent/traces") return jsonResponse({ traces: [], storage_available: true, truncated: false });
    if (path === "/api/live/settings") return jsonResponse(liveSettingsSnapshot());
    if (path === "/api/obs/settings") return jsonResponse({ enabled: false, websocket_url: "ws://127.0.0.1:4455", password_configured: false, storage_available: true });
    if (path === "/api/live") return jsonResponse(liveSnapshot());
    if (path === "/api/resources") return jsonResponse(resourceSnapshot());
    if (path === "/api/obs") return jsonResponse({ connected: false, recording: false, current_scene: "", scenes: [] });
    if (path === "/api/llm") return jsonResponse(llmSnapshot);
    return jsonResponse(serverStatus());
  });
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  expect(screen.queryByRole("heading", { name: "文字播报" })).not.toBeInTheDocument();
  expect(fetcher).not.toHaveBeenCalled();
  finish({ config_path: "desktop.toml", server_url: "http://127.0.0.1:19777", server: { ready: true, managed: true, last_error: null, log_path: "server.log" }, runtime: { running: true, simulation: true, last_error: null } });
  await userEvent.click(await screen.findByRole("link", { name: "语音播报" }));
  await screen.findByRole("heading", { name: "文字播报" });
  for (const name of ["角色与人物卡", "Agent 互动", "Agent 观察", "LLM 接入", "直播连接", "OBS 控制", "声音训练"]) {
    await userEvent.click(screen.getByRole("link", { name }));
  }
  await waitFor(() => expect(fetcher.mock.calls.length).toBeGreaterThanOrEqual(6));
  expect(fetcher.mock.calls.every(([url]) => String(url).startsWith("http://127.0.0.1:19777/"))).toBe(true);
  expect(screen.getByText(/静音模拟/)).toBeInTheDocument();
});

it("shows desktop initialization errors without starting panels at a fallback address", async () => {
  vi.mocked(getDesktopStatus).mockRejectedValue(new Error("IPC unavailable"));
  const fetcher = vi.fn<typeof fetch>();
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  expect(await screen.findByRole("alert")).toHaveTextContent("桌面配置读取失败");
  expect(fetcher).not.toHaveBeenCalled();
});

it("waits for the bundled service and displays startup errors without polling business APIs", async () => {
  vi.mocked(getDesktopStatus).mockResolvedValue({ config_path: "desktop.toml", server_url: "http://127.0.0.1:19777",
    server: { ready: false, managed: false, last_error: "主服务端口已被占用", log_path: "server.log" },
    runtime: { running: true, simulation: true, last_error: null } });
  const fetcher = vi.fn<typeof fetch>();
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  expect((await screen.findAllByText("主服务端口已被占用"))[0]).toBeVisible();
  expect(screen.getByRole("switch", { name: "主服务" })).toBeEnabled();
  expect(screen.getAllByRole("switch")).toHaveLength(3);
  await userEvent.click(screen.getByRole("link", { name: "语音播报" }));
  expect(await screen.findByRole("heading", { name: "正在等待主服务" })).toBeInTheDocument();
  expect(fetcher).not.toHaveBeenCalled();
});

it("keeps native lifecycle logs visible while the bundled server HTTP is offline", async () => {
  window.history.replaceState(null, "", "#logs");
  vi.mocked(getDesktopStatus).mockResolvedValue({ config_path: "desktop.toml", server_url: "http://127.0.0.1:19777",
    server: { ready: false, managed: true, last_error: "raw-secret", log_path: "server.log" },
    runtime: { running: false, simulation: false, last_error: "raw-secret" },
    runtime_logs: { entries: [{ id: "native-safe-1", timestamp: "2026-10-03T00:00:00Z", level: "error", source: "desktop", category: "lifecycle", code: "server_failed", summary: "桌面主服务启动失败" }], storage_available: false, truncated: false },
  });
  vi.stubGlobal("fetch", vi.fn<typeof fetch>().mockRejectedValue(new Error("connection refused raw-secret")));
  render(<App />);
  expect(await screen.findByText("桌面主服务启动失败")).toBeVisible();
  expect(screen.getByRole("region", { name: "日志列表" })).not.toHaveTextContent("raw-secret");
  expect(screen.queryByRole("heading", { name: "正在等待主服务" })).not.toBeInTheDocument();
});

// Native IPC is the boundary: the page must send real service commands and refresh state.
it("stops and restarts the native execution service through the overview switch", async () => {
  const status = { config_path: "desktop.toml", server_url: "http://127.0.0.1:19777",
    server: { ready: true, managed: true, last_error: null, log_path: "server.log" },
    runtime: { running: true, simulation: false, last_error: null } };
  vi.mocked(getDesktopStatus).mockImplementation(async () => structuredClone(status));
  vi.mocked(setDesktopServiceEnabled).mockImplementation(async (id, enabled) => {
    if (id !== "windows") throw new Error("wrong service");
    status.runtime.running = enabled;
  });
  vi.stubGlobal("fetch", vi.fn(async () => jsonResponse({ enabled: false, authenticated: false })));
  render(<App />);
  const toggle = await screen.findByRole("switch", { name: "Windows 执行端" });
  expect(toggle).toHaveAttribute("aria-checked", "true");
  await userEvent.click(toggle);
  await waitFor(() => expect(toggle).toHaveAttribute("aria-checked", "false"));
  expect(screen.queryByText("请关闭并重新启动桌面程序。")).not.toBeInTheDocument();
  await userEvent.click(toggle);
  await waitFor(() => expect(toggle).toHaveAttribute("aria-checked", "true"));
});

it("protects external services and explains missing TTS setup", async () => {
  vi.mocked(getDesktopStatus).mockResolvedValue({ config_path: "desktop.toml", server_url: "http://127.0.0.1:19777",
    server: { ready: true, managed: false, last_error: null, log_path: "server.log" },
    runtime: { running: true, simulation: false, last_error: null },
    environment: { phase: "idle", busy: false, message: "检测完成", logs: [], distros: [], selectedDistro: null,
      backend: { ready: false, engineRoot: "", pythonPath: "", modelRoot: "", gpu: false, detail: "" }, models: [], progress: 0, inferenceRunning: false },
  });
  render(<App />);
  expect(await screen.findByRole("switch", { name: "主服务" })).toBeDisabled();
  expect(screen.getByRole("switch", { name: "TTS 语音引擎" })).toBeDisabled();
  expect(screen.getByText(/请先在环境与模型页安装/)).toBeVisible();
});

it("can stop TTS after stopping the main service and blocks it during environment work", async () => {
  const status = { config_path: "desktop.toml", server_url: "http://127.0.0.1:19777",
    server: { ready: false, stopped: true, managed: true, last_error: null, log_path: "server.log" },
    runtime: { running: true, simulation: false, last_error: null },
    environment: { phase: "running", busy: false, message: "推理运行中", logs: [], distros: [], selectedDistro: "Ubuntu",
      backend: { ready: true, engineRoot: "/engine", pythonPath: "/python", modelRoot: "/models", gpu: true, detail: "就绪" },
      models: [{ id: "gpt-sovits-v2", name: "GPT-SoVITS v2", capability: "training_inference", selected: true, downloaded: true }], progress: 100, inferenceRunning: true },
  };
  vi.mocked(getDesktopStatus).mockImplementation(async () => structuredClone(status));
  vi.mocked(setDesktopServiceEnabled).mockImplementation(async (id, enabled) => {
    if (id !== "tts" || enabled) throw new Error("wrong command");
    status.environment.inferenceRunning = false;
    status.environment.busy = true;
    status.environment.message = "正在停止推理";
  });
  render(<App />);
  const toggle = await screen.findByRole("switch", { name: "TTS 语音引擎" });
  expect(toggle).toBeEnabled();
  await userEvent.click(toggle);
  await waitFor(() => expect(toggle).toBeDisabled());
  expect(screen.getByText("正在停止推理")).toBeVisible();
  expect(screen.getByRole("switch", { name: "主服务" })).toBeDisabled();
});
