import { expect, it, vi } from "vitest";
import { createObsClient } from "./obs";

it("rejects malformed snapshots and reports HTTP failures without replaying actions", async () => {
  const fetcher = vi.fn<typeof fetch>().mockResolvedValue(Response.json({ connected: true, recording: false, current_scene: "main", scenes: [] }));
  await expect(createObsClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "invalid_response" });
  fetcher.mockResolvedValue(Response.json({ code: "obs_failed", message: "OBS 无响应" }, { status: 502 }));
  await expect(createObsClient({ fetcher }).execute({ type: "start_recording" })).rejects.toThrow("OBS 无响应");
  expect(fetcher).toHaveBeenCalledTimes(2);
});

it("times out a request and a response body even when its promise ignores cancellation", async () => {
  for (const fetcher of [async () => new Promise<Response>(() => {}), async () => ({ ok: true, json: () => new Promise(() => {}) }) as Response]) {
    await expect(createObsClient({ fetcher, timeoutMs: 10 }).getStatus()).rejects.toMatchObject({ code: "request_timeout" });
  }
});

it("sends OBS credentials only in the settings POST body and returns a redacted settings snapshot", async () => {
  const calls: { url: string; method: string; body: unknown }[] = [];
  const client = createObsClient({ fetcher: async (url, init) => {
    calls.push({ url: String(url), method: init?.method ?? "GET", body: init?.body ? JSON.parse(String(init.body)) : null });
    return Response.json({ enabled: true, websocket_url: "ws://127.0.0.1:4455", password_configured: true, storage_available: true, password: "server-must-not-return-this" });
  } });
  const saved = await client.saveSettings({ enabled: true, websocket_url: "ws://127.0.0.1:4455", password: "private-password", clear_password: false });
  expect(saved).not.toHaveProperty("password");
  await client.getSettings();
  expect(calls).toEqual([
    { url: "http://127.0.0.1:19600/api/obs/settings", method: "POST", body: { enabled: true, websocket_url: "ws://127.0.0.1:4455", password: "private-password", clear_password: false } },
    { url: "http://127.0.0.1:19600/api/obs/settings", method: "GET", body: null },
  ]);
});

it("rejects malformed OBS settings before they become editable state", async () => {
  const client = createObsClient({ fetcher: async () => Response.json({ enabled: true, websocket_url: "ws://127.0.0.1:4455", password_configured: "secret", storage_available: true }) });
  await expect(client.getSettings()).rejects.toMatchObject({ code: "invalid_response" });
});

it("accepts expanded IPv6 loopback addresses returned by the execution host", async () => {
  const client = createObsClient({ fetcher: async () => Response.json({ enabled: true, websocket_url: "ws://[0:0:0:0:0:0:0:1]:4455", password_configured: false, storage_available: true }) });
  await expect(client.getSettings()).resolves.toMatchObject({ websocket_url: "ws://[0:0:0:0:0:0:0:1]:4455" });
});
