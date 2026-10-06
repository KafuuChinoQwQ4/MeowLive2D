import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { launcherSnapshot } from "../test/launcher-fixtures";
import { jsonResponse, serverStatus } from "../test/server-fixtures";
import { App } from "./App";

afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); window.history.replaceState(null, "", "/"); });

it("keeps business APIs unavailable until the user starts the main service", async () => {
  vi.stubEnv("VITE_MEOWLIVE_LAUNCHER", "true");
  let status = launcherSnapshot("starting");
  const businessCalls: string[] = [];
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async url => {
    const path = String(url);
    if (path === "/api/desktop/status") return new Response("not found", { status: 404 });
    if (path === "/api/launcher/status") return jsonResponse(status);
    if (path.endsWith("/api/admin/session")) return jsonResponse({ enabled: false, authenticated: false });
    businessCalls.push(path);
    return jsonResponse(serverStatus());
  }));
  render(<App />);
  await screen.findByText("启动中");
  expect(screen.getByRole("switch", { name: "主服务" })).toBeChecked();
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
    if (path === "/api/desktop/status") return new Response("not found", { status: 404 });
    if (path === "/api/launcher/models") return jsonResponse(modelLibrarySnapshot());
    return jsonResponse(launcherSnapshot());
  }));
  render(<App />);
  await userEvent.click(await screen.findByRole("link", { name: "环境与模型" }));
  expect(await screen.findByRole("heading", { name: "选择本地语音模型" })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "前往启动与运行" })).not.toBeInTheDocument();
  expect(requests.every(path => path.startsWith("/api/launcher/") || path === "/api/desktop/status")).toBe(true);
  await userEvent.click(screen.getByRole("tab", { name: /下载模型/ }));
  await userEvent.type(screen.getByRole("searchbox", { name: "搜索语音模型" }), "GPT");
  await userEvent.click(screen.getByRole("link", { name: "启动与运行" }));
  await userEvent.click(screen.getByRole("link", { name: "环境与模型" }));
  expect(screen.getByRole("searchbox", { name: "搜索语音模型" })).toHaveValue("GPT");
});


it("shows the execution client status in the workspace status bar", async () => {
  vi.stubEnv("VITE_MEOWLIVE_LAUNCHER", "true");
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async () => jsonResponse(launcherSnapshot("running", "running", "running"))));
  render(<App />);
  expect(await screen.findByText("Windows 执行端 · 已连接")).toBeInTheDocument();
  expect(screen.getAllByRole("switch")).toHaveLength(3);
});

it("shows browser startup cards without a build-time launcher flag", async () => {
  vi.stubEnv("VITE_MEOWLIVE_LAUNCHER", "false");
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async url => String(url) === "/api/desktop/status"
    ? new Response("not found", { status: 404 }) : jsonResponse(launcherSnapshot("running", "running", "running"))));
  render(<App />);
  expect(await screen.findByRole("switch", { name: "主服务" })).toBeInTheDocument();
  expect(screen.getByRole("switch", { name: "TTS 语音引擎" })).toBeInTheDocument();
  expect(screen.getByRole("switch", { name: "Windows 执行端" })).toBeInTheDocument();
});

it("controls the packaged App from a browser and exposes its model downloads", async () => {
  const token = "b".repeat(64);
  const environment = { phase: "idle", busy: false, message: "环境任务完成", logs: [], distros: [{ name: "archlinux", version: 2 }], selectedDistro: "archlinux", backend: { ready: true, gpu: true, engineRoot: "/engine", pythonPath: "/python", modelRoot: "/models", detail: "" }, models: [{ id: "gpt-sovits-v2", name: "GPT-SoVITS v2", capability: "training_inference", downloaded: true, selected: true }, { id: "cosyvoice3", name: "CosyVoice3", capability: "download_only", downloaded: false, selected: false }], progress: 0, inferenceRunning: true };
  const status = { platform: "windows", config_path: "desktop.toml", server_url: "http://127.0.0.1:19600", server: { ready: true, managed: true, last_error: null, log_path: "server.log" }, runtime: { running: true, simulation: false, last_error: null }, environment };
  const fetcher = vi.fn<typeof fetch>(async (url, init) => {
    if (String(url) === "/api/desktop/status") return jsonResponse({ status, token });
    if (String(url) === "/api/desktop/command") {
      const body = JSON.parse(String(init?.body));
      if (body.command === "environment_status") return jsonResponse({ result: environment });
      return jsonResponse({ result: null });
    }
    return jsonResponse({ enabled: false, authenticated: false });
  });
  vi.stubGlobal("fetch", fetcher);
  render(<App />);
  await screen.findByRole("switch", { name: "主服务" });
  expect(screen.getAllByRole("switch")).toHaveLength(3);
  await userEvent.click(screen.getByRole("switch", { name: "Windows 执行端" }));
  expect(fetcher).toHaveBeenCalledWith("/api/desktop/command", expect.objectContaining({ body: JSON.stringify({ command: "desktop_service_set_enabled", args: { id: "windows", enabled: false } }) }));
  await userEvent.click(screen.getAllByRole("link", { name: "环境与模型" })[0]);
  expect(await screen.findByRole("button", { name: "下载 CosyVoice3" })).toBeEnabled();
  await userEvent.click(screen.getByRole("button", { name: "下载 CosyVoice3" }));
  expect(fetcher).toHaveBeenCalledWith("/api/desktop/command", expect.objectContaining({ headers: expect.objectContaining({ "X-MeowLive-Desktop-Token": token }), body: JSON.stringify({ command: "environment_action", args: { request: { action: "download_model", distro: "archlinux", modelId: "cosyvoice3" } } }) }));
});
