import { afterEach, describe, expect, it, vi } from "vitest";
import { createRuntimeLogClient, readRuntimeLogs } from "./logs";
afterEach(() => vi.useRealTimers());

describe("runtime logs client", () => {
  it("accepts a full 1000-entry response", () => {
    const entries = Array.from({ length: 1000 }, (_, index) => ({ id: String(index), timestamp: "2026-10-03T00:00:00Z", level: "info", source: "server", category: "runtime", code: "started", summary: "服务已就绪" }));
    expect(readRuntimeLogs({ entries, storage_available: true, truncated: false }).entries).toHaveLength(1000);
  });
  it("filters and validates bounded log responses", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response(JSON.stringify({ storage_available: true, truncated: false, entries: [{ id: "1", timestamp: "2026-10-03T00:00:00.000Z", level: "warn", source: "browser", category: "runtime", code: "browser_error", summary: "安全摘要" }] }), { status: 200 }));
    const client = createRuntimeLogClient({ baseUrl: "http://logs.test", fetcher });
    const result = await client.list({ level: "warn", query: "错误", limit: 10 });
    expect(result.entries[0]?.code).toBe("browser_error");
    expect(String(fetcher.mock.calls[0]?.[0])).toContain("level=warn");
  });

  it("reports only the constrained code payload", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response("{}", { status: 200 }));
    const client = createRuntimeLogClient({ baseUrl: "http://logs.test", fetcher });
    const untrusted = { code: "request_failed" as const, source: "secret-path", category: "raw" };
    await client.report(untrusted);
    expect(JSON.parse(String(fetcher.mock.calls[0]?.[1]?.body))).toEqual({ code: "request_failed" });
  });

  it("times out even if transport ignores cancellation", async () => {
    vi.useFakeTimers();
    const client = createRuntimeLogClient({ fetcher: () => new Promise(() => {}), timeoutMs: 20 });
    const result = client.list().catch(error => error);
    await vi.advanceTimersByTimeAsync(21);
    expect(await Promise.race([result, Promise.resolve("pending")])).toMatchObject({ code: "request_timeout" });
  });

  it("rejects malformed timestamps and oversized responses", async () => {
    const entry = { id: "1", timestamp: "not-a-date", level: "warn", source: "browser", category: "runtime", code: "browser_error", summary: "摘要" };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValueOnce(new Response(JSON.stringify({ entries: [entry], storage_available: true, truncated: false })))
      .mockResolvedValueOnce(new Response(JSON.stringify({ entries: Array.from({ length: 1001 }, () => ({ ...entry, timestamp: "2026-10-03T00:00:00Z" })), storage_available: true, truncated: false })));
    const client = createRuntimeLogClient({ fetcher });
    await expect(client.list()).rejects.toMatchObject({ code: "invalid_response" });
    await expect(client.list()).rejects.toMatchObject({ code: "invalid_response" });
  });
});
