import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AgentObservabilityClient } from "../../services/server/agent-observability";
import { AgentObservabilityPanel } from "./AgentObservabilityPanel";

const summary = (id = "trace-new", updated = 1_200) => ({ id, status: "running" as const, trigger: "live_events",
  started_at_ms: 1_000, updated_at_ms: updated, finished_at_ms: null, event_count: 1, turn_count: 1, tool_count: 1,
  speech_id: "speech-1", result: "等待播放", truncated: false });
const trace = (id = "trace-new") => ({ summary: summary(id), events: [{ id: "event-1", kind: "chat", viewer: "小猫", summary: "晚上好" }],
  turns: [{ id: `turn-${id}`, index: 1, tool_round: 0, retry_attempt: 0, provider: "custom", api_format: "openai_chat",
    model: "test-model", status: "completed" as const, started_at_ms: 1_000, first_token_ms: 20, finished_at_ms: 1_100,
    latency_ms: 100, usage: { input_tokens: 10, output_tokens: 2, cache_read_tokens: 0, cache_write_tokens: 0, reasoning_tokens: 0 } }],
  steps: [{ sequence: 1, occurred_at_ms: 1_050, kind: "tool_finished" as const, status: "completed" as const,
    message: "网页搜索完成", turn_id: `turn-${id}`, tool_name: "web_search", speech_id: null, elapsed_ms: 30,
    sources: ["https://example.com/article"] }, { sequence: 2, occurred_at_ms: 1_100, kind: "speech_playing" as const,
    status: "running" as const, message: "桌面执行端正在播放", turn_id: null, tool_name: null, speech_id: "speech-1",
    elapsed_ms: null, sources: [] }] });

function client(overrides: Partial<AgentObservabilityClient> = {}): AgentObservabilityClient {
  return {
    baseUrl: "http://localhost:19600",
    getScheduler: vi.fn(async () => ({ ready: false, phase: "speaking" as const, block_reason: "speech_in_flight" as const,
      message: "正在等待本轮语音完成", remaining_ms: null, pending_events: 2, deciding_events: 0, active_speeches: 1, updated_at_ms: 1_200 })),
    listTraces: vi.fn(async () => ({ traces: [summary(), summary("trace-old", 900)], storage_available: false, truncated: true })),
    getTrace: vi.fn(async id => trace(id)),
    ...overrides,
  };
}

describe("Agent 观察页面", () => {
  it("展示调度原因、turn、工具来源、播放步骤和存储边界", async () => {
    render(<AgentObservabilityPanel client={client()} pollIntervalMs={10_000} />);
    expect(await screen.findByText("正在等待本轮语音完成")).toBeVisible();
    expect(await screen.findByRole("heading", { name: "Trace trace-new" })).toBeVisible();
    expect(screen.getByRole("button", { name: /trace-new/ }).querySelector("time")).toHaveAttribute("dateTime", new Date(1_000).toISOString());
    expect(screen.getByText("第 1 次模型调用")).toBeVisible();
    expect(screen.getByText("网页搜索完成")).toBeVisible();
    expect(screen.getByText("桌面执行端正在播放")).toBeVisible();
    expect(screen.getByRole("link", { name: "example.com" })).toHaveAttribute("href", "https://example.com/article");
    expect(screen.getByText(/历史持久化暂不可用/)).toBeVisible();
    expect(screen.getByText(/列表只显示最近一部分/)).toBeVisible();
    expect(screen.queryByText(/PRIVATE_PROMPT|PRIVATE_TOOL_BODY/)).not.toBeInTheDocument();
  });

  it("用户选择历史 trace 后，列表刷新不会抢回最新项", async () => {
    vi.useFakeTimers();
    let lists = 0;
    const api = client({ listTraces: vi.fn(async () => {
      lists += 1;
      return { traces: lists === 1
        ? [summary("trace-new", 1_200), summary("trace-old", 900)]
        : [summary("trace-new", 1_200 + lists)], storage_available: true, truncated: lists > 1 };
    }) });
    render(<AgentObservabilityPanel client={api} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    fireEvent.click(screen.getByRole("button", { name: /trace-old/ }));
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByRole("heading", { name: "Trace trace-old" })).toBeVisible();
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });
    expect(screen.getByRole("heading", { name: "Trace trace-old" })).toBeVisible();
  });

  it("未显式选择时跟随最新 trace", async () => {
    vi.useFakeTimers();
    let calls = 0;
    const api = client({ listTraces: vi.fn(async () => ({
      traces: ++calls === 1 ? [summary("first")] : [summary("second"), summary("first")],
      storage_available: true, truncated: false,
    })) });
    render(<AgentObservabilityPanel client={api} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByRole("heading", { name: "Trace first" })).toBeVisible();
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });
    expect(screen.getByRole("heading", { name: "Trace second" })).toBeVisible();
  });

  it("同一 trace 的旧轮询响应不会覆盖较新的手动响应", async () => {
    let resolveOld: (value: ReturnType<typeof trace>) => void = () => {};
    let resolveNew: (value: ReturnType<typeof trace>) => void = () => {};
    let calls = 0;
    const api = client({
      listTraces: vi.fn(async () => ({ traces: [summary()], storage_available: true, truncated: false })),
      getTrace: vi.fn(() => {
        calls += 1;
        return new Promise<ReturnType<typeof trace>>(resolve => { if (calls === 1) resolveOld = resolve; else resolveNew = resolve; });
      }),
    });
    render(<AgentObservabilityPanel client={api} pollIntervalMs={10_000} />);
    const button = await screen.findByRole("button", { name: /trace-new/ });
    fireEvent.click(button);
    await act(async () => {
      const newer = trace(); newer.summary.result = "较新的详情"; resolveNew(newer);
    });
    expect(await screen.findByText("较新的详情")).toBeVisible();
    await act(async () => {
      const older = trace(); older.summary.result = "过期的详情"; resolveOld(older);
    });
    expect(screen.getByText("较新的详情")).toBeVisible();
    expect(screen.queryByText("过期的详情")).not.toBeInTheDocument();
  });

  it("隐藏和卸载页面时取消请求，重新可见后恢复", async () => {
    vi.useFakeTimers();
    const signals: AbortSignal[] = [];
    const pending = () => (_queryOrSignal?: unknown, possibleSignal?: AbortSignal) => {
      const signal = possibleSignal ?? _queryOrSignal as AbortSignal;
      signals.push(signal);
      return new Promise<never>(() => {});
    };
    const api = client({ getScheduler: pending() as AgentObservabilityClient["getScheduler"],
      listTraces: pending() as AgentObservabilityClient["listTraces"] });
    const view = render(<section hidden><AgentObservabilityPanel client={api} pollIntervalMs={100} /></section>);
    await act(async () => { await vi.advanceTimersByTimeAsync(500); });
    expect(signals).toHaveLength(0);
    view.rerender(<section><AgentObservabilityPanel client={api} pollIntervalMs={100} /></section>);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(signals).toHaveLength(2);
    view.rerender(<section hidden><AgentObservabilityPanel client={api} pollIntervalMs={100} /></section>);
    await act(async () => {});
    expect(signals.every(signal => signal.aborted)).toBe(true);
    view.unmount();
  });
});
