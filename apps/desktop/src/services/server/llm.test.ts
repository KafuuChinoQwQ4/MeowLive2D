import { describe, expect, it, vi } from "vitest";
import type { LlmModelsRequest, LlmSettings, LlmSettingsRequest, LlmSettingsSnapshot } from "@meowlive/contracts";
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
  reasoning_effort: "default",
};
const snapshot: LlmSettingsSnapshot = {
  settings,
  key_configured: true,
  restart_required: false,
  active_model: "gpt-4o-mini",
  storage_available: true,
  profiles: [],
  selected_profile_id: null,
};
const request: LlmSettingsRequest = { settings, api_key: null, clear_api_key: false };
const modelsRequest: LlmModelsRequest = {
  provider: "openai", api_format: "openai_responses", base_url: "https://api.openai.com/v1",
  mode: "cloud", api_key: null, clear_api_key: false,
};

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

  it("通过独立接口管理已保存配置且只读取公开摘要", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const fetcher = vi.fn<typeof fetch>(async (url, init) => {
      calls.push({ path: String(url), body: init?.body ? JSON.parse(String(init.body)) : undefined });
      return new Response(JSON.stringify(snapshot));
    });
    const client = createLlmClient({ baseUrl: "http://localhost:19600", fetcher });
    await client.createProfile({ name: "DeepSeek", settings, api_key: "private", clear_api_key: false });
    await client.selectProfile({ id: "profile-id" });
    await client.renameProfile({ id: "profile-id", name: "我的 DeepSeek" });
    await client.deleteProfile({ id: "profile-id" });
    expect(calls.map(call => call.path)).toEqual([
      "http://localhost:19600/api/llm/profiles",
      "http://localhost:19600/api/llm/profiles/select",
      "http://localhost:19600/api/llm/profiles/rename",
      "http://localhost:19600/api/llm/profiles/delete",
    ]);
    expect(calls[0].body).toMatchObject({ name: "DeepSeek", api_key: "private" });
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

  it("获取供应商模型时发送连接草稿，不需要模型名或保存配置", async () => {
    const calls: Array<{ path: string; init?: RequestInit }> = [];
    const result = { base_url: "https://api.openai.com/v1", models: [{ id: "gpt-4.1-mini", name: "GPT 4.1 Mini" }] };
    const client = createLlmClient({ baseUrl: "http://localhost:19600", fetcher: async (url, init) => {
      calls.push({ path: String(url), init });
      return new Response(JSON.stringify(result));
    } });
    await expect(client.listModels({ ...modelsRequest, api_key: "draft-key" })).resolves.toEqual(result);
    expect(calls).toHaveLength(1);
    expect(calls[0].path).toBe("http://localhost:19600/api/llm/models");
    expect(calls[0].init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0].init?.body))).toEqual({ ...modelsRequest, api_key: "draft-key" });
  });

  it.each([
    { base_url: "https://example.com/v1", models: [{ id: "same", name: "A" }, { id: "same", name: "B" }] },
    { base_url: "https://example.com/v1", models: [{ id: "猫".repeat(43), name: "long" }] },
    { base_url: "https://example.com/v1", models: [{ id: "model", name: "猫".repeat(86) }] },
    { base_url: "https://example.com/v1", models: [{ id: "model\n", name: "name" }] },
    { base_url: "https://example.com/v1", models: [{ id: "model", name: "name", api_key: "leaked" }] },
    { base_url: "file:///local", models: [] },
    { base_url: "https://secret@example.com/v1", models: [] },
    { base_url: "https://example.com/v1?key=secret", models: [] },
    { base_url: "https://example.com/" + "a".repeat(4096), models: [] },
    { base_url: "https://example.com/v1", models: Array.from({ length: 1001 }, (_, i) => ({ id: `model-${i}`, name: `Model ${i}` })) },
  ])("拒绝无效、重复、过大或含额外字段的模型列表 %#", async result => {
    const client = createLlmClient({ fetcher: async () => new Response(JSON.stringify(result)) });
    await expect(client.listModels(modelsRequest)).rejects.toMatchObject({ code: "invalid_response" });
  });

  it("空模型列表保持为空，不合成模型选项", async () => {
    const result = { base_url: "https://api.openai.com/v1", models: [] };
    const client = createLlmClient({ fetcher: async () => new Response(JSON.stringify(result)) });
    await expect(client.listModels(modelsRequest)).resolves.toEqual(result);
  });

  it("模型获取有独立超时并支持调用方取消，即使传输忽略信号", async () => {
    vi.useFakeTimers();
    try {
      let requestSignal: AbortSignal | undefined;
      const client = createLlmClient({ fetcher: async (_url, init) => {
        requestSignal = init?.signal ?? undefined;
        return new Promise<Response>(() => {});
      } });
      const pending = client.listModels(modelsRequest);
      const assertion = expect(pending).rejects.toMatchObject({ code: "request_timeout" });
      await vi.advanceTimersByTimeAsync(31_999);
      expect(requestSignal?.aborted).toBe(false);
      await vi.advanceTimersByTimeAsync(1);
      await assertion;
      const controller = new AbortController();
      const canceled = client.listModels(modelsRequest, controller.signal);
      controller.abort();
      await expect(canceled).rejects.toMatchObject({ name: "AbortError" });
    } finally { vi.useRealTimers(); }
  });
});
