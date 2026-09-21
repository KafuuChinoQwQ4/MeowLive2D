import { describe, expect, it } from "vitest";
import { activitySnapshot, runtimeSnapshot, usageSnapshot } from "../../features/llm-runtime/runtime-fixtures";
import { createLlmRuntimeClient } from "./llm-runtime";

describe("Agent 运行服务边界", () => {
  it("发送日期和模型筛选，读取真实的可空用量并保存显式密钥操作", async () => {
    const calls: Array<{ url: string; body: unknown }> = [];
    const client = createLlmRuntimeClient({ baseUrl: "http://localhost:19600/", fetcher: async (url, init) => {
      calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : null });
      return new Response(JSON.stringify(String(url).includes("/usage") ? usageSnapshot() : runtimeSnapshot()));
    } });
    expect((await client.getUsage({ since_ms: 100, until_ms: 200, provider: "custom", model: "model/a + b" })).records[0].estimated_cost_microusd).toBeNull();
    expect(new URL(calls[0].url).searchParams.get("model")).toBe("model/a + b");
    expect(new URL(calls[0].url).searchParams.get("since_ms")).toBe("100");
    await client.saveSettings({ settings: runtimeSnapshot().settings, search_api_key: null, clear_search_api_key: true });
    expect(calls[1].url).toBe("http://localhost:19600/api/agent/runtime");
    expect(calls[1].body).toMatchObject({ search_api_key: null, clear_search_api_key: true });
  });

  it.each([
    ["settings", { ...runtimeSnapshot(), search_api_key: "LEAKED_SECRET" }],
    ["settings", { ...runtimeSnapshot(), settings: { ...runtimeSnapshot().settings, max_tool_rounds: 99 } }],
    ["settings", { ...runtimeSnapshot(), settings: { ...runtimeSnapshot().settings, search_endpoint: "http://search.example.com" } }],
    ["usage", { ...usageSnapshot(), totals: { ...usageSnapshot().totals, calls: -1 } }],
    ["usage", { ...usageSnapshot(), records: [{ ...usageSnapshot().records[0], estimated_cost_microusd: "0" }] }],
    ["activity", { ...activitySnapshot(), tools: [{ ...activitySnapshot().tools[0], sources: ["javascript:alert(1)"] }] }],
    ["activity", { ...activitySnapshot(), thinking: "LEAKED_SECRET" }],
  ])("拒绝错误或敏感字段响应 %s %#", async (kind, value) => {
    const client = createLlmRuntimeClient({ fetcher: async () => new Response(JSON.stringify(value)) });
    const pending = kind === "settings" ? client.getSettings() : kind === "usage" ? client.getUsage() : client.getActivity();
    await expect(pending).rejects.toMatchObject({ code: "invalid_response" });
  });

  it("超时覆盖未结束的响应体，卸载取消即使传输忽略 signal", async () => {
    const client = createLlmRuntimeClient({ timeoutMs: 10, fetcher: async () => new Response(new ReadableStream()) });
    await expect(client.getActivity()).rejects.toMatchObject({ code: "request_timeout" });
    const cancel = new AbortController();
    const pending = client.getUsage({}, cancel.signal);
    cancel.abort();
    await expect(pending).rejects.toMatchObject({ name: "AbortError" });
  });

  it("错误响应不向界面回显供应商内容或密钥", async () => {
    const client = createLlmRuntimeClient({ fetcher: async () => new Response(JSON.stringify({ code: "failed", message: "LEAKED_SECRET" }), { status: 500 }) });
    await expect(client.getSettings()).rejects.toThrow("运行配置请求失败");
    await expect(client.getSettings()).rejects.not.toThrow("LEAKED_SECRET");
  });
});
