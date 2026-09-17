import { describe, expect, it, vi } from "vitest";
import type { LlmSettings, LlmSettingsRequest, LlmSettingsSnapshot } from "@meowlive/contracts";
import { createLlmClient, readLlmSnapshot } from "./llm";

const settings: LlmSettings = {
  provider: "openai",
  api_format: "openai_responses",
  base_url: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  mode: "cloud",
  timeout_seconds: 30,
  max_tokens: 1024,
  json_mode: true,
};
const snapshot: LlmSettingsSnapshot = {
  settings,
  key_configured: true,
  restart_required: false,
  active_model: "gpt-4o-mini",
  storage_available: true,
};
const request: LlmSettingsRequest = { settings, api_key: null, clear_api_key: false };

describe("LLM 服务边界", () => {
  it("使用配置路由，并原样发送显式的密钥操作", async () => {
    const calls: Array<{ path: string; init?: RequestInit }> = [];
    const fetcher = vi.fn<typeof fetch>(async (url, init) => {
      calls.push({ path: String(url), init });
      return new Response(JSON.stringify(String(url).endsWith("/test") ? { message: "连接成功" } : snapshot));
    });
    const client = createLlmClient({ baseUrl: "http://localhost:19600/", fetcher });

    await client.getSettings();
    await client.saveSettings({ ...request, api_key: "new-secret" });
    await expect(client.testSettings({ ...request, clear_api_key: true })).resolves.toEqual({ message: "连接成功" });

    expect(calls.map(call => [call.path, call.init?.method])).toEqual([
      ["http://localhost:19600/api/llm/settings", "GET"],
      ["http://localhost:19600/api/llm/settings", "POST"],
      ["http://localhost:19600/api/llm/test", "POST"],
    ]);
    expect(JSON.parse(String(calls[1].init?.body))).toEqual({ ...request, api_key: "new-secret" });
    expect(JSON.parse(String(calls[2].init?.body))).toEqual({ ...request, clear_api_key: true });
  });

  it("拒绝未知格式、越界参数和泄漏密钥的响应", () => {
    expect(readLlmSnapshot(snapshot)).toEqual(snapshot);
    expect(readLlmSnapshot({ ...snapshot, settings: { ...settings, api_format: "openai_responses" } }).settings.api_format).toBe("openai_responses");
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, api_format: "other" } })).toThrow("无效");
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, timeout_seconds: 0 } })).toThrow("无效");
    expect(() => readLlmSnapshot({ ...snapshot, api_key: "leaked" })).toThrow("无效");
  });

  it("按服务端 UTF-8 字节上限校验地址和模型", () => {
    const longUrl = `https://example.com/${"a".repeat(4076)}`;
    expect(new TextEncoder().encode(longUrl)).toHaveLength(4096);
    expect(readLlmSnapshot({ ...snapshot, settings: { ...settings, base_url: longUrl } }).settings.base_url).toBe(longUrl);
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, base_url: `${longUrl}a` } })).toThrow("无效");
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, model: "猫".repeat(43) } })).toThrow("无效");
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, provider: "" } })).toThrow("无效");
  });

  it("即使传输不响应 AbortSignal，也能处理超时和调用方取消", async () => {
    const client = createLlmClient({ fetcher: vi.fn(() => new Promise<Response>(() => {})), timeoutMs: 10 });
    await expect(client.getSettings()).rejects.toThrow("超时");
    const controller = new AbortController();
    const pending = client.getSettings(controller.signal);
    controller.abort();
    await expect(pending).rejects.toMatchObject({ name: "AbortError" });
  });

  it("保留服务端错误信息", async () => {
    const client = createLlmClient({ fetcher: vi.fn().mockResolvedValue(new Response(
      JSON.stringify({ code: "llm_test_failed", message: "供应商拒绝了密钥" }), { status: 400 },
    )) });
    await expect(client.testSettings(request)).rejects.toThrow("供应商拒绝了密钥");
  });
});
