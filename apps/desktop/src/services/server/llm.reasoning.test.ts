import { describe, expect, it } from "vitest";
import { createLlmClient, readLlmSnapshot } from "./llm";

const request = { provider: "openai", api_format: "openai_responses", model: "gpt-5", reasoning_effort: "ultra", max_tokens: 4096 };
const result = { requested: "ultra", effective: "high", supported: ["minimal", "low", "medium", "high"], strategy: "openai_effort", budget_tokens: null, note: "已映射到模型最高档 high。", error: null };

describe("LLM 推理预览契约", () => {
  it("只发送不含密钥的推理参数并读取模型映射", async () => {
    const calls: Array<{ url: string; init?: RequestInit }> = [];
    const client = createLlmClient({ fetcher: async (url, init) => {
      calls.push({ url: String(url), init });
      return new Response(JSON.stringify(result));
    } });
    await expect(client.previewReasoning(request)).resolves.toEqual(result);
    expect(calls).toHaveLength(1);
    expect(calls[0].url).toBe("http://127.0.0.1:19600/api/llm/reasoning");
    expect(calls[0].init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0].init?.body))).toEqual(request);
  });

  it.each([
    { requested: "super" }, { requested: "low" }, { effective: "super" }, { effective: "default" },
    { effective: "ultra" }, { supported: ["low", "low", "high"] }, { supported: ["high", "low"] },
    { supported: ["default", "low", "high"] }, { supported: ["low", "super"] },
    { budget_tokens: -1 }, { budget_tokens: 0 }, { budget_tokens: 1.5 }, { budget_tokens: 3585 },
    { effective: null, budget_tokens: 1024 }, { api_key: "leaked" }, { error: false }, { note: null },
  ])("拒绝未知档位、无序能力和无效预算 %#", async change => {
    const client = createLlmClient({ fetcher: async () => new Response(JSON.stringify({ ...result, ...change })) });
    await expect(client.previewReasoning(request)).rejects.toMatchObject({ code: "invalid_response" });
  });

  it("接受 default、未知模型和预算不足，并支持取消请求", async () => {
    for (const value of [
      { ...result, requested: "default", effective: null, note: "保留模型默认行为。" },
      { ...result, requested: "default", effective: "default", note: "保留模型默认行为。" },
      { ...result, effective: null, supported: [], strategy: "unsupported", note: "未知模型，不覆盖推理参数。" },
      { ...result, budget_tokens: 3584 },
      { ...result, error: "请提高最大输出至至少 1536 tokens。" },
    ]) {
      const client = createLlmClient({ fetcher: async () => new Response(JSON.stringify(value)) });
      await expect(client.previewReasoning({ ...request, reasoning_effort: value.requested })).resolves.toEqual(value);
    }
    const client = createLlmClient({ fetcher: async () => new Promise(() => {}) });
    const controller = new AbortController();
    const pending = client.previewReasoning(request, controller.signal);
    controller.abort();
    await expect(pending).rejects.toMatchObject({ name: "AbortError" });
  });

  it("读取八档和云端 65536 上限，拒绝未知档位及本地超额配置", () => {
    const settings = { provider: "openai", api_format: "openai_responses", base_url: "", model: "", mode: "cloud", timeout_seconds: 30, max_tokens: 65536, json_mode: true, reasoning_effort: "default" };
    const snapshot = { settings, key_configured: false, restart_required: false, active_model: "", storage_available: true };
    for (const reasoning_effort of ["default", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"]) {
      expect(readLlmSnapshot({ ...snapshot, settings: { ...settings, reasoning_effort } }).settings.reasoning_effort).toBe(reasoning_effort);
    }
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, reasoning_effort: "super" } })).toThrow("无效");
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, max_tokens: 65537 } })).toThrow("无效");
    expect(() => readLlmSnapshot({ ...snapshot, settings: { ...settings, mode: "local", max_tokens: 1025 } })).toThrow("无效");
  });
});
