import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { launcherSnapshot } from "../test/launcher-fixtures";
import { jsonResponse, serverStatus } from "../test/server-fixtures";
import { App } from "./App";

afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); window.history.replaceState(null, "", "/"); });

it("waits for automatic startup and hides business APIs when the service goes down", async () => {
  vi.stubEnv("VITE_MEOWLIVE_LAUNCHER", "true");
  let status = launcherSnapshot("starting");
  const businessCalls: string[] = [];
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async url => {
    const path = String(url);
    if (path === "/api/launcher/status") return jsonResponse(status);
    if (path.endsWith("/api/admin/session")) return jsonResponse({ enabled: false, authenticated: false });
    businessCalls.push(path);
    return jsonResponse(serverStatus());
  }));
  render(<App />);
  await screen.findByText("启动中");
  expect(screen.queryByRole("switch", { name: "主服务" })).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("link", { name: "语音播报" }));
  expect(await screen.findByRole("button", { name: "前往启动与运行" })).toBeInTheDocument();
  expect(businessCalls).toHaveLength(0);
  status = launcherSnapshot("running");
  await userEvent.click(screen.getByRole("button", { name: "前往启动与运行" }));
  await userEvent.click(screen.getByRole("button", { name: "刷新服务状态" }));
  await userEvent.click(screen.getByRole("link", { name: "语音播报" }));
  expect(await screen.findByRole("heading", { name: "文字播报" })).toBeInTheDocument();
  expect(businessCalls.length).toBeGreaterThan(0);
  status = launcherSnapshot("failed");
  await userEvent.click(screen.getByRole("link", { name: "启动与运行" }));
  await userEvent.click(screen.getByRole("button", { name: "刷新服务状态" }));
  await waitFor(() => expect(screen.queryByRole("heading", { name: "文字播报" })).not.toBeInTheDocument());
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
  expect(screen.getAllByRole("switch")).toHaveLength(2);
});
