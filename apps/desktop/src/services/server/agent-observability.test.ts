import { describe, expect, it } from "vitest";
import { createAgentObservabilityClient } from "./agent-observability";

const summary = () => ({ id: "trace-1", status: "running", trigger: "live_events", started_at_ms: 1_000,
  updated_at_ms: 1_200, finished_at_ms: null, event_count: 1, turn_count: 1, tool_count: 1,
  speech_id: "speech-1", result: "等待播放", truncated: false });
const scheduler = () => ({ ready: false, phase: "speaking", block_reason: "speech_in_flight", message: "正在等待本轮语音完成",
  remaining_ms: null, pending_events: 2, deciding_events: 0, active_speeches: 1, updated_at_ms: 1_200 });
const trace = () => ({ summary: summary(), events: [{ id: "event-1", kind: "chat", viewer: "小猫", summary: "晚上好" }],
  turns: [{ id: "turn-1", index: 1, tool_round: 0, retry_attempt: 0, provider: "custom", api_format: "openai_chat",
    model: "test-model", status: "completed", started_at_ms: 1_000, first_token_ms: 20, finished_at_ms: 1_100, latency_ms: 100,
    usage: { input_tokens: 10, output_tokens: 2, cache_read_tokens: 0, cache_write_tokens: 0, reasoning_tokens: 0 } }],
  steps: [{ sequence: 1, occurred_at_ms: 1_050, kind: "tool_finished", status: "completed", message: "网页搜索完成",
    turn_id: "turn-1", tool_name: "web_search", speech_id: null, elapsed_ms: 30, sources: ["https://example.com/article"] }] });

describe("Agent 观察服务边界", () => {
  it("发送有界历史查询并读取调度、列表和详情", async () => {
    const urls: string[] = [];
    const client = createAgentObservabilityClient({ baseUrl: "http://localhost:19600/", fetcher: async url => {
      urls.push(String(url));
      const path = new URL(String(url)).pathname;
      return new Response(JSON.stringify(path.endsWith("/scheduler") ? scheduler()
        : path.endsWith("/traces") ? { traces: [summary()], storage_available: true, truncated: false } : trace()));
    } });
    expect((await client.getScheduler()).block_reason).toBe("speech_in_flight");
    expect((await client.listTraces({ limit: 25, before_ms: 2_000 })).traces[0].id).toBe("trace-1");
    expect((await client.getTrace("trace-1")).turns[0].usage.output_tokens).toBe(2);
    expect(new URL(urls[1]).searchParams.get("limit")).toBe("25");
    expect(new URL(urls[1]).searchParams.get("before_ms")).toBe("2000");
    expect(urls[2]).toBe("http://localhost:19600/api/agent/traces/trace-1");
  });

  it.each([
    ["scheduler", { ...scheduler(), block_reason: "secret_reason" }],
    ["list", { traces: Array.from({ length: 101 }, summary), storage_available: true, truncated: false }],
    ["trace", { ...trace(), private_prompt: "PRIVATE_PROMPT" }],
    ["trace", { ...trace(), turns: Array.from({ length: 17 }, () => trace().turns[0]) }],
    ["trace", { ...trace(), steps: [{ ...trace().steps[0], sources: ["javascript:alert(1)"] }] }],
  ])("拒绝畸形或敏感的 %s 响应", async (kind, value) => {
    const client = createAgentObservabilityClient({ fetcher: async () => new Response(JSON.stringify(value)) });
    const pending = kind === "scheduler" ? client.getScheduler() : kind === "list" ? client.listTraces() : client.getTrace("trace-1");
    await expect(pending).rejects.toMatchObject({ code: "invalid_response" });
  });

  it("编码 trace ID，并区分 HTTP、超时和调用方取消", async () => {
    let requested = "";
    const http = createAgentObservabilityClient({ fetcher: async url => {
      requested = String(url);
      return new Response(JSON.stringify({ code: "not_found", message: "PRIVATE_UPSTREAM" }), { status: 404 });
    } });
    await expect(http.getTrace("trace with space")).rejects.toThrow("Trace 详情请求失败");
    expect(requested).toContain("trace%20with%20space");
    await expect(http.getTrace("trace with space")).rejects.not.toThrow("PRIVATE_UPSTREAM");
    const timeout = createAgentObservabilityClient({ timeoutMs: 10, fetcher: async () => new Response(new ReadableStream()) });
    await expect(timeout.getScheduler()).rejects.toMatchObject({ code: "request_timeout" });
    const cancel = new AbortController();
    const pending = timeout.listTraces({}, cancel.signal);
    cancel.abort();
    await expect(pending).rejects.toMatchObject({ name: "AbortError" });
  });
});
