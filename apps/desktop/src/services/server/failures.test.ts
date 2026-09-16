import { describe, expect, it, vi } from "vitest";
import { jsonResponse, serverStatus } from "../../test/server-fixtures";
import { createServerClient } from "./index";

describe("主服务错误与取消", () => {
  it("保留服务端错误码与可读原因", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ code: "bridge_unavailable", message: "桌面执行端未连接" }, 503));
    await expect(createServerClient({ fetcher }).submitSpeech({ text: "你好", voice_id: "default" })).rejects.toMatchObject({
      code: "bridge_unavailable", message: "桌面执行端未连接", status: 503,
    });
  });

  it("非 JSON 的服务失败仍包含 HTTP 状态", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(new Response("upstream unavailable", { status: 502 }));
    await expect(createServerClient({ fetcher }).getStatus()).rejects.toMatchObject({ status: 502, message: expect.stringContaining("502") });
  });

  it("网络失败提供可辨认的连接错误", async () => {
    const fetcher = vi.fn<typeof fetch>().mockRejectedValue(new TypeError("Failed to fetch"));
    await expect(createServerClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "connection_failed", message: expect.stringContaining("连接") });
  });

  it("拒绝协议版本不兼容的服务", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(serverStatus({ protocol_version: 99 })));
    await expect(createServerClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "protocol_mismatch" });
  });

  it("拒绝缺少字段的成功响应", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ bridge_connected: true }));
    await expect(createServerClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "invalid_response" });
  });

  it("调用方取消会中断 HTTP 请求并保留 AbortError", async () => {
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => new Promise((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true });
    }));
    const controller = new AbortController();
    const request = createServerClient({ fetcher }).getStatus(controller.signal);
    const assertion = expect(request).rejects.toMatchObject({ name: "AbortError" });
    controller.abort();
    await assertion;
  });

  it("超时会终止请求并提示超时", async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => new Promise((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true });
    }));
    const request = createServerClient({ fetcher, timeoutMs: 100 }).getStatus();
    const assertion = expect(request).rejects.toMatchObject({ code: "request_timeout", message: expect.stringContaining("超时") });
    await Promise.all([assertion, vi.advanceTimersByTimeAsync(100)]);
  });
});
