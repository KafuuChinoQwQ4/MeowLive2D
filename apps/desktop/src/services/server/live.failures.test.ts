import type { LiveConnectionSnapshot } from "@meowlive/contracts";
import { describe, expect, it, vi } from "vitest";
import { liveSettingsSnapshot, liveSnapshot } from "../../test/live-fixtures";
import { deferred, jsonResponse } from "../../test/server-fixtures";
import { createLiveClient } from "./live";

describe("直播连接响应校验", () => {
  it.each([
    { app_id: 123 },
    { app_id: "9223372036854775808" },
    { app_id: "1e5" },
    { enabled: "yes" },
    { identity_code_configured: null },
    { access_key_id_configured: "true" },
    { access_key_secret_configured: undefined },
    { storage_available: undefined },
  ])("拒绝格式错误的配置快照 %j", async (overrides) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ ...liveSettingsSnapshot(), ...overrides }));
    await expect(createLiveClient({ fetcher }).getSettings()).rejects.toMatchObject({ code: "invalid_response" });
  });

  it.each([
    ["未知阶段", liveSnapshot({ phase: "waiting" as LiveConnectionSnapshot["phase"] })],
    ["空平台", liveSnapshot({ platform: "" })],
    ["非布尔配置状态", { ...liveSnapshot(), configured: "yes" }],
    ["非字符串房间", { ...liveSnapshot(), room_id: 123 }],
    ["空房间", liveSnapshot({ room_id: "" })],
    ["过长平台", liveSnapshot({ platform: "p".repeat(33) })],
    ["过长房间", liveSnapshot({ room_id: "r".repeat(129) })],
    ["负数接收计数", liveSnapshot({ accepted_events: -1 })],
    ["小数去重计数", liveSnapshot({ duplicate_events: 1.5 })],
    ["溢出丢弃计数", liveSnapshot({ rejected_events: 0x1_0000_0000 })],
    ["非有限重连次数", liveSnapshot({ reconnect_attempts: Number.NaN })],
    ["非字符串错误", { ...liveSnapshot(), last_error: false }],
    ["过长错误", liveSnapshot({ last_error: "e".repeat(2001) })],
    ["缺少字段", (() => { const value: Partial<LiveConnectionSnapshot> = liveSnapshot(); delete value.platform; return value; })()],
  ])("拒绝%s", async (_label, value) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
    await expect(createLiveClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "invalid_response" });
  });
});

describe("直播连接请求失败与取消", () => {
  it("保留服务端连接错误和 HTTP 状态", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ code: "live_not_configured", message: "直播平台尚未配置" }, 409));
    await expect(createLiveClient({ fetcher }).connect()).rejects.toMatchObject({
      code: "live_not_configured", message: "直播平台尚未配置", status: 409,
    });
  });

  it("非 JSON 的服务失败仍包含 HTTP 状态", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response("upstream unavailable", { status: 502 }));
    await expect(createLiveClient({ fetcher }).getStatus()).rejects.toMatchObject({ status: 502, message: expect.stringContaining("502") });
  });

  it("网络失败提供可辨认的连接错误", async () => {
    const fetcher = vi.fn<typeof fetch>().mockRejectedValue(new TypeError("Failed to fetch"));
    await expect(createLiveClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "connection_failed", message: expect.stringContaining("连接") });
  });

  it("调用方取消会中断请求并保留 AbortError", async () => {
    const pending = deferred<Response>();
    let requestSignal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => {
      requestSignal = init?.signal;
      return pending.promise;
    });
    const controller = new AbortController();
    const request = createLiveClient({ fetcher }).getStatus(controller.signal);

    controller.abort();

    await expect(request).rejects.toMatchObject({ name: "AbortError" });
    expect(requestSignal?.aborted).toBe(true);
  });

  it("超时会中止无响应请求并返回可辨认错误", async () => {
    vi.useFakeTimers();
    let requestSignal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => {
      requestSignal = init?.signal;
      return new Promise<Response>(() => undefined);
    });
    const request = createLiveClient({ fetcher, timeoutMs: 100 }).getStatus();
    const assertion = expect(request).rejects.toMatchObject({ code: "request_timeout", message: expect.stringContaining("超时") });

    await Promise.all([assertion, vi.advanceTimersByTimeAsync(100)]);
    expect(requestSignal?.aborted).toBe(true);
  });
});
