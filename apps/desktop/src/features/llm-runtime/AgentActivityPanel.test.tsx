import { act, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createLlmRuntimeClient } from "../../services/server/llm-runtime";
import { activitySnapshot } from "./runtime-fixtures";
import { AgentActivityPanel } from "./AgentActivityPanel";

describe("Agent 实时活动", () => {
  it("更新接收与工具状态和安全来源，不显示私有模型文本", async () => {
    vi.useFakeTimers();
    let calls = 0;
    const client = createLlmRuntimeClient({ fetcher: async () => new Response(JSON.stringify(++calls === 1 ? activitySnapshot()
      : { ...activitySnapshot(), phase: "receiving", tools: [{ ...activitySnapshot().tools[0], status: "completed" }] })) });
    render(<AgentActivityPanel client={client} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByText("正在查资料")).toBeVisible();
    expect(screen.getByRole("link", { name: "example.com" })).toHaveAttribute("href", "https://example.com/article");
    expect(screen.queryByText("PRIVATE_MODEL_TEXT")).not.toBeInTheDocument();
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });
    expect(screen.getByText("正在接收回复")).toBeVisible();
    expect(screen.getByText(/已完成 ·/)).toBeVisible();
  });

  it("页面隐藏或卸载取消请求，重新可见后恢复且不重叠请求", async () => {
    vi.useFakeTimers();
    const signals: AbortSignal[] = [];
    const client = createLlmRuntimeClient({ fetcher: async (_url, init) => {
      signals.push(init!.signal!);
      return new Promise<Response>(() => {});
    } });
    const view = render(<section hidden><AgentActivityPanel client={client} pollIntervalMs={100} /></section>);
    await act(async () => { await vi.advanceTimersByTimeAsync(500); });
    expect(signals).toHaveLength(0);
    view.rerender(<section><AgentActivityPanel client={client} pollIntervalMs={100} /></section>);
    await act(async () => { await vi.advanceTimersByTimeAsync(500); });
    expect(signals).toHaveLength(1);
    view.rerender(<section hidden><AgentActivityPanel client={client} pollIntervalMs={100} /></section>);
    await act(async () => {});
    expect(signals[0].aborted).toBe(true);
    view.rerender(<section><AgentActivityPanel client={client} pollIntervalMs={100} /></section>);
    await act(async () => {});
    expect(signals).toHaveLength(2);
    view.unmount();
    expect(signals[1].aborted).toBe(true);
    await act(async () => { await vi.advanceTimersByTimeAsync(500); });
    expect(signals).toHaveLength(2);
  });
});
