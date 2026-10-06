import { afterEach, describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { getDesktopStatus, readDesktopStatus, setDesktopServiceEnabled } from "./index";
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); vi.resetAllMocks(); });

const server = { ready: true, managed: true, last_error: null, log_path: "server.log" };

describe("desktop status boundary", () => {
  it("browser mode needs no native bridge", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("not found", { status: 404 })));
    expect(await getDesktopStatus()).toBeNull();
  });

  it("uses the App host from a browser and authenticates service actions", async () => {
    const status = { server, config_path: "desktop.toml", server_url: "http://127.0.0.1:19600", runtime: { running: true, simulation: false, last_error: null } };
    const token = "a".repeat(64);
    const fetcher = vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({ status, token })))
      .mockResolvedValueOnce(new Response(JSON.stringify({ result: null })));
    vi.stubGlobal("fetch", fetcher);
    expect(await getDesktopStatus()).toMatchObject(status);
    await setDesktopServiceEnabled("tts", true);
    expect(fetcher).toHaveBeenLastCalledWith("/api/desktop/command", expect.objectContaining({
      method: "POST", headers: expect.objectContaining({ "X-MeowLive-Desktop-Token": token }),
      body: JSON.stringify({ command: "desktop_service_set_enabled", args: { id: "tts", enabled: true } }),
    }));
  });

  it("preserves the configured server and refuses malformed native status", () => {
    expect(readDesktopStatus({ server, config_path: "C:/Users/test/desktop.toml", server_url: "http://192.168.1.10:19600", runtime: { running: true, simulation: false, last_error: null } })).toEqual({ server, config_path: "C:/Users/test/desktop.toml", server_url: "http://192.168.1.10:19600", runtime: { running: true, simulation: false, last_error: null } });
    expect(() => readDesktopStatus({ server, config_path: "x", server_url: "javascript:alert(1)", runtime: { running: true, simulation: false, last_error: null } })).toThrow();
    expect(() => readDesktopStatus({ server, config_path: "x", server_url: "http://localhost:19600", runtime: { running: "yes" } })).toThrow();
  });

  it("accepts native Linux status without a Windows runtime and rejects unknown platforms", () => {
    const linux = readDesktopStatus({ platform: "linux", config_path: "/home/test/.config/meowlive/server.toml", server_url: "http://127.0.0.1:19600", server, environment: { phase: "idle", busy: false, message: "Linux 首版暂不包含本机语音执行端", logs: [], distros: [], selectedDistro: null, backend: { ready: false, gpu: false, engineRoot: "", pythonPath: "", modelRoot: "", detail: "" }, models: [], progress: 0, inferenceRunning: false } });
    expect(linux.platform).toBe("linux");
    expect(linux.runtime.running).toBe(false);
    expect(() => readDesktopStatus({ platform: "freebsd", config_path: "x", server_url: "http://localhost:19600", server })).toThrow();
  });

  it("refuses credentials, query strings and malformed native origins", () => {
    for (const server_url of ["http://user:secret@localhost:19600", "http://localhost:19600?key=private", "http://localhost:19600#private", "http://localhost:bad", "http://localhost:19600 "]) {
      expect(() => readDesktopStatus({ server, config_path: "desktop.toml", server_url, runtime: { running: true, simulation: false, last_error: null } })).toThrow();
    }
  });

  it("bounds a native IPC request that never resolves", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("__TAURI_INTERNALS__", {});
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}));
    const pending = expect(getDesktopStatus()).rejects.toThrow("桌面状态读取超时");
    await vi.advanceTimersByTimeAsync(5_000);
    await pending;
  });
});

it("preserves optional structured desktop logs and rejects malformed entries", () => {
  const status = { server, config_path: "desktop.toml", server_url: "http://localhost:19600", runtime: { running: false, simulation: false, last_error: "secret" } };
  const runtime_logs = { entries: [{ id: "desktop-1", timestamp: "2026-10-03T00:00:00Z", level: "error", source: "desktop", category: "runtime", code: "runtime_stopped", summary: "桌面执行端已停止" }], storage_available: false, truncated: false };
  expect(readDesktopStatus({ ...status, runtime_logs })).toHaveProperty("runtime_logs", runtime_logs);
  expect(() => readDesktopStatus({ ...status, runtime_logs: { ...runtime_logs, entries: [{ ...runtime_logs.entries[0], level: "SECRET" }] } })).toThrow();
});

it("observes native state transitions with bounded safe logs and stable timestamps", async () => {
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  const status = { config_path: "transition-test.toml", server_url: "http://localhost:19605", server: { ...server, ready: false, last_error: "token=secret" }, runtime: { running: false, simulation: false, last_error: "prompt=private" } };
  vi.mocked(invoke).mockResolvedValue(status);
  const first = await getDesktopStatus();
  expect(first?.runtime_logs?.entries).toHaveLength(2);
  expect(first?.runtime_logs?.entries.map(entry => entry.code)).toEqual(expect.arrayContaining(["desktop_server_failed", "desktop_runtime_failed"]));
  expect(JSON.stringify(first?.runtime_logs)).not.toMatch(/secret|private/);
  expect((await getDesktopStatus())?.runtime_logs).toEqual(first?.runtime_logs);
  vi.mocked(invoke).mockResolvedValue({ ...status, server: { ...server }, runtime: { running: true, simulation: false, last_error: null } });
  const recovered = await getDesktopStatus();
  expect(recovered?.runtime_logs?.entries).toHaveLength(4);
  expect(recovered?.runtime_logs?.entries.map(entry => entry.code)).toEqual(expect.arrayContaining(["desktop_server_ready", "desktop_runtime_running"]));
  for (let index = 0; index < 102; index++) {
    vi.mocked(invoke).mockResolvedValue({ ...status, runtime: { running: index % 2 === 0, simulation: false, last_error: null } });
    await getDesktopStatus();
  }
  const bounded = await getDesktopStatus();
  expect(bounded?.runtime_logs?.entries).toHaveLength(100);
  expect(bounded?.runtime_logs?.truncated).toBe(true);
  expect(bounded?.runtime_logs?.storage_available).toBe(false);
});


it("shows actionable WSL startup diagnostics in the control-panel logs without raw output", async () => {
  vi.stubGlobal("__TAURI_INTERNALS__", {});
  const status = { config_path: "wsl-timeout-test.toml", server_url: "http://localhost:19600", server: { ...server, stopped: false, ready: false, last_error: "Wsl/Service/0x8007274c token=private" }, runtime: { running: false, simulation: false, last_error: null } };
  vi.mocked(invoke).mockResolvedValue(status);
  const logs = (await getDesktopStatus())?.runtime_logs?.entries;
  expect(logs?.find(entry => entry.code === "desktop_wsl_timeout")?.summary).toContain("wsl --shutdown");
  expect(JSON.stringify(logs)).not.toContain("private");
});
