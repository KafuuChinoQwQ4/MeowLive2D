import { afterEach, describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { getDesktopStatus, readDesktopStatus } from "./index";
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); vi.resetAllMocks(); });

const server = { ready: true, managed: true, last_error: null, log_path: "server.log" };

describe("desktop status boundary", () => {
  it("browser mode needs no native bridge", async () => {
    expect(await getDesktopStatus()).toBeNull();
  });

  it("preserves the configured server and refuses malformed native status", () => {
    expect(readDesktopStatus({ server, config_path: "C:/Users/test/desktop.toml", server_url: "http://192.168.1.10:19600", runtime: { running: true, simulation: false, last_error: null } })).toEqual({ server, config_path: "C:/Users/test/desktop.toml", server_url: "http://192.168.1.10:19600", runtime: { running: true, simulation: false, last_error: null } });
    expect(() => readDesktopStatus({ server, config_path: "x", server_url: "javascript:alert(1)", runtime: { running: true, simulation: false, last_error: null } })).toThrow();
    expect(() => readDesktopStatus({ server, config_path: "x", server_url: "http://localhost:19600", runtime: { running: "yes" } })).toThrow();
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
