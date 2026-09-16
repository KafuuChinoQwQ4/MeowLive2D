import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { agentStatus } from "../test/agent-fixtures";
import { liveSnapshot } from "../test/live-fixtures";
import { resourceSnapshot } from "../test/resource-fixtures";
import { jsonResponse, serverStatus } from "../test/server-fixtures";

vi.mock("../services/desktop", () => ({ getDesktopStatus: vi.fn() }));
import { getDesktopStatus } from "../services/desktop";
import { App } from "./App";

afterEach(() => { vi.unstubAllGlobals(); vi.resetAllMocks(); window.history.replaceState(null, "", "/"); });

it("waits for desktop configuration and uses its address in every panel", async () => {
  let finish!: (value: Awaited<ReturnType<typeof getDesktopStatus>>) => void;
  vi.mocked(getDesktopStatus).mockReturnValue(new Promise((resolve) => { finish = resolve; }));
  const fetcher = vi.fn<typeof fetch>(async (url) => {
    const path = new URL(String(url)).pathname;
    if (path === "/api/agent") return jsonResponse(agentStatus());
    if (path === "/api/live") return jsonResponse(liveSnapshot());
    if (path === "/api/resources") return jsonResponse(resourceSnapshot());
    if (path === "/api/obs") return jsonResponse({ connected: false, recording: false, current_scene: "", scenes: [] });
    return jsonResponse(serverStatus());
  });
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  expect(screen.queryByRole("heading", { name: "文字播报" })).not.toBeInTheDocument();
  expect(fetcher).not.toHaveBeenCalled();
  finish({ config_path: "desktop.toml", server_url: "http://127.0.0.1:19777", runtime: { running: true, simulation: true, last_error: null } });
  await userEvent.click(await screen.findByRole("link", { name: "语音播报" }));
  await screen.findByRole("heading", { name: "文字播报" });
  for (const name of ["角色与音色", "Agent 互动", "直播连接", "OBS 控制", "训练与离线"]) {
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
