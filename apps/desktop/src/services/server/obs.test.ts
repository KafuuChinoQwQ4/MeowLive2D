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
