import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { agentStatus } from "../test/agent-fixtures";
import { liveSettingsSnapshot, liveSnapshot } from "../test/live-fixtures";
import { resourceSnapshot } from "../test/resource-fixtures";
import { jsonResponse, serverStatus } from "../test/server-fixtures";

vi.mock("../services/desktop", () => ({ getDesktopStatus: vi.fn() }));
import { getDesktopStatus } from "../services/desktop";
import { App } from "./App";

const llmSnapshot = { settings: { provider: "custom", api_format: "openai_responses", base_url: "", model: "", mode: "cloud", timeout_seconds: 30, max_tokens: 1024, json_mode: true }, key_configured: false, restart_required: false, active_model: "", storage_available: true };

afterEach(() => { vi.unstubAllGlobals(); vi.resetAllMocks(); window.history.replaceState(null, "", "/"); });

it("waits for desktop configuration and uses its address in every panel", async () => {
  let finish!: (value: Awaited<ReturnType<typeof getDesktopStatus>>) => void;
  vi.mocked(getDesktopStatus).mockReturnValue(new Promise((resolve) => { finish = resolve; }));
  const fetcher = vi.fn<typeof fetch>(async (url) => {
    const path = new URL(String(url)).pathname;
    if (path === "/api/admin/session") return jsonResponse({ enabled: false, authenticated: false });
    if (path === "/api/agent") return jsonResponse(agentStatus());
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
  finish({ config_path: "desktop.toml", server_url: "http://127.0.0.1:19777", runtime: { running: true, simulation: true, last_error: null } });
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
