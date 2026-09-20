import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { launcherSnapshot } from "../test/launcher-fixtures";
import { jsonResponse, serverStatus } from "../test/server-fixtures";
import { agentStatus } from "../test/agent-fixtures";
import { resourceSnapshot } from "../test/resource-fixtures";
import { liveSettingsSnapshot, liveSnapshot } from "../test/live-fixtures";
import { App } from "./App";

afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); window.history.replaceState(null, "", "/"); });

it("the launcher survives stopping the main service and does not poll inactive business APIs", async () => {
  vi.stubEnv("VITE_MEOWLIVE_LAUNCHER", "true");
  let status = launcherSnapshot();
  const businessCalls: string[] = [];
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async (url, init) => {
    const path = String(url);
    if (path === "/api/launcher/services/server") {
      status = launcherSnapshot(JSON.parse(String(init?.body)).enabled ? "running" : "stopped");
      return jsonResponse(status);
    }
    if (path === "/api/launcher/status") return jsonResponse(status);
    if (path.endsWith("/api/admin/session")) return jsonResponse({ enabled: false, authenticated: false });
    businessCalls.push(path);
    if (path.endsWith("/api/agent")) return jsonResponse(agentStatus());
    if (path.endsWith("/api/live/settings")) return jsonResponse(liveSettingsSnapshot());
    if (path.endsWith("/api/live")) return jsonResponse(liveSnapshot());
    if (path.endsWith("/api/resources")) return jsonResponse(resourceSnapshot());
    return jsonResponse(serverStatus());
  }));
  render(<App />);
  const toggle = await screen.findByRole("switch", { name: "主服务" });
  await waitFor(() => expect(toggle).toBeEnabled());
  expect(businessCalls).toHaveLength(0);
  await userEvent.click(screen.getByRole("link", { name: "语音播报" }));
  expect(await screen.findByRole("button", { name: "前往启动与运行" })).toBeInTheDocument();
  expect(businessCalls).toHaveLength(0);
  await userEvent.click(screen.getByRole("button", { name: "前往启动与运行" }));
  await userEvent.click(toggle);
  await userEvent.click(screen.getByRole("link", { name: "语音播报" }));
  expect(await screen.findByRole("heading", { name: "文字播报" })).toBeInTheDocument();
  expect(businessCalls.length).toBeGreaterThan(0);
  await userEvent.click(screen.getByRole("link", { name: "启动与运行" }));
  await userEvent.click(toggle);
  await waitFor(() => expect(screen.queryByRole("heading", { name: "文字播报" })).not.toBeInTheDocument());
  expect(screen.getByRole("switch", { name: "主服务" })).toBeEnabled();
  expect(screen.getByRole("heading", { name: "启动与运行" })).toBeInTheDocument();
});

it("environment and models remain accessible while the main service is stopped", async () => {
  vi.stubEnv("VITE_MEOWLIVE_LAUNCHER", "true");
  const { modelLibrarySnapshot } = await import("../test/model-library-fixtures");
  const requests: string[] = [];
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async url => {
    const path = String(url); requests.push(path);
    if (path === "/api/launcher/models") return jsonResponse(modelLibrarySnapshot());
    return jsonResponse(launcherSnapshot());
  }));
  render(<App />);
  await userEvent.click(await screen.findByRole("link", { name: "环境与模型" }));
  expect(await screen.findByRole("heading", { name: "选择本地语音模型" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "前往启动与运行" })).not.toBeInTheDocument();
  expect(requests.every(path => path.startsWith("/api/launcher/"))).toBe(true);
  await userEvent.click(screen.getByRole("tab", { name: /下载模型/ }));
  await userEvent.type(screen.getByRole("searchbox", { name: "搜索语音模型" }), "GPT");
  await userEvent.click(screen.getByRole("link", { name: "启动与运行" }));
  await userEvent.click(screen.getByRole("link", { name: "环境与模型" }));
  expect(screen.getByRole("searchbox", { name: "搜索语音模型" })).toHaveValue("GPT");
});


it("shows the execution client connection in the workspace status bar", async () => {
  vi.stubEnv("VITE_MEOWLIVE_LAUNCHER", "true");
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async () => jsonResponse(launcherSnapshot("running", "running", "running"))));
  render(<App />);
  expect(await screen.findByText("Windows 执行端 · 已连接")).toBeInTheDocument();
  expect(screen.getAllByRole("switch")).toHaveLength(3);
});
