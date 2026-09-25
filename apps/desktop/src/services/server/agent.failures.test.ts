import { interactionSettings } from "../../test/agent-fixtures";
import { describe, expect, it, vi } from "vitest";
import type { AgentEventSnapshot, AgentSnapshot } from "@meowlive/contracts";
import { deferred, jsonResponse } from "../../test/server-fixtures";
import { createAgentClient } from "./agent";

function event(overrides: Partial<AgentEventSnapshot> = {}): AgentEventSnapshot {
  return {
    event: { id: "event-1", source: "simulator", viewer: "小猫", kind: { type: "gift", name: "小鱼干", count: 2 } },
    status: "queued",
    speech_id: "speech-1",
    error: null,
    ...overrides,
  };
}

function snapshot(overrides: Partial<AgentSnapshot> = {}): AgentSnapshot {
  return {
    paused: false,
    phase: "waiting",
    settings: { persona: "猫娘", system_prompt: "", topic: "游戏", proactive_enabled: false, cooldown_ms: 30_000, interaction: { ...interactionSettings } },
    events: [event()],
    last_error: null,
    current_speech_id: null,
    llm_configured: true,
    bridge_connected: true,
    ...overrides,
  };
}

describe("Agent 响应校验", () => {
  it.each([
    ["未知阶段", snapshot({ phase: "resting" as AgentSnapshot["phase"] })],
    ["非有限冷却", snapshot({ settings: { ...snapshot().settings, cooldown_ms: Number.NaN } })],
    ["未知事件状态", snapshot({ events: [event({ status: "lost" as AgentEventSnapshot["status"] })] })],
    ["无效事件载荷", snapshot({ events: [event({ event: { ...event().event, kind: { type: "gift", name: "鱼", count: -1 } } })] })],
    ["缺少布尔字段", (() => { const value = snapshot() as Partial<AgentSnapshot>; delete value.llm_configured; return value; })()],
    ["超过历史上限", snapshot({ events: Array.from({ length: 4_097 }, (_, index) => event({ event: { ...event().event, id: `event-${index}` } })) })],
  ])("拒绝%s", async (_label, value) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
    await expect(createAgentClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "invalid_response" });
  });

  it("拒绝损坏的事件提交结果", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ accepted: -1, duplicates: "1" }, 202));
    await expect(createAgentClient({ fetcher }).submitEvents({ events: [] })).rejects.toMatchObject({ code: "invalid_response" });
  });
});

describe("Agent 请求失败与取消", () => {
  it("保留服务端恢复条件错误", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ code: "llm_not_configured", message: "请先完成本地 LLM 配置" }, 409));
    await expect(createAgentClient({ fetcher }).resume()).rejects.toMatchObject({
      code: "llm_not_configured", message: "请先完成本地 LLM 配置", status: 409,
    });
  });

  it("调用方取消会中断请求并保留 AbortError", async () => {
    const pending = deferred<Response>();
    let requestSignal: AbortSignal | null | undefined;
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => {
      requestSignal = init?.signal;
      return pending.promise;
    });
    const controller = new AbortController();
    const request = createAgentClient({ fetcher }).getStatus(controller.signal);

    controller.abort();

    await expect(request).rejects.toMatchObject({ name: "AbortError" });
    expect(requestSignal?.aborted).toBe(true);
  });

  it("超时会终止请求并返回可辨认错误", async () => {
    vi.useFakeTimers();
    const fetcher = vi.fn<typeof fetch>().mockImplementation((_url, init) => new Promise((_resolve, reject) => {
      init?.signal?.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true });
    }));
    const request = createAgentClient({ fetcher, timeoutMs: 100 }).getStatus();
    const assertion = expect(request).rejects.toMatchObject({ code: "request_timeout", message: expect.stringContaining("超时") });

    await Promise.all([assertion, vi.advanceTimersByTimeAsync(100)]);
  });
});
