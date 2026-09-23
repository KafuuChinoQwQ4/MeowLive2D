import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AgentClient } from "../../services/server/agent";
import { agentStatus } from "../../test/agent-fixtures";
import { deferred } from "../../test/server-fixtures";
import { AgentPanel } from "./AgentPanel";

function client(overrides: Partial<AgentClient> = {}): AgentClient {
  return {
    baseUrl: "http://127.0.0.1:19600",
    getStatus: vi.fn().mockResolvedValue(agentStatus()),
    saveSettings: vi.fn().mockResolvedValue(agentStatus()),
    pause: vi.fn().mockResolvedValue(agentStatus()),
    resume: vi.fn().mockResolvedValue(agentStatus({ paused: false, phase: "waiting", llm_configured: true, bridge_connected: true })),
    submitEvents: vi.fn().mockResolvedValue({ accepted: 1, duplicates: 0 }),
    ...overrides,
  };
}

describe("Agent 面板状态与控制", () => {
  it("保存互动设置时读取最新人物卡，避免覆盖角色页的修改", async () => {
    const initial = agentStatus();
    const latest = agentStatus({ settings: { ...initial.settings, persona: "角色页新人物卡" } });
    const api = client({ getStatus: vi.fn().mockResolvedValueOnce(initial).mockResolvedValue(latest) });
    render(<AgentPanel client={api} />);
    await screen.findByText("Agent 已暂停");
    fireEvent.change(screen.getByLabelText("直播话题"), { target: { value: "新的互动话题" } });
    fireEvent.click(screen.getByRole("button", { name: "保存设置并暂停" }));
    await waitFor(() => expect(api.saveSettings).toHaveBeenCalledWith(expect.objectContaining({
      persona: "角色页新人物卡", topic: "新的互动话题",
    }), expect.any(AbortSignal)));
  });

  it("默认暂停且未配置模型时给出本地配置指引并禁止恢复", async () => {
    render(<AgentPanel client={client()} />);

    expect(await screen.findByText("Agent 已暂停")).toBeVisible();
    expect(screen.getByText("LLM 未配置")).toBeVisible();
    expect(screen.getByText("桌面执行端未连接")).toBeVisible();
    expect(screen.getByRole("link", { name: "前往 LLM 接入" })).toHaveAttribute("href", "#llm");
    expect(screen.getByRole("button", { name: "恢复 Agent" })).toBeDisabled();
  });

  it("可恢复的 Agent 执行恢复并切换到等待状态", async () => {
    const ready = agentStatus({ llm_configured: true, bridge_connected: true });
    const api = client({ getStatus: vi.fn().mockResolvedValue(ready) });
    render(<AgentPanel client={api} />);
    await screen.findByText("Agent 已暂停");

    fireEvent.click(screen.getByRole("button", { name: "恢复 Agent" }));

    expect(await screen.findByText("Agent 等待事件")).toBeVisible();
  });

  it("运行中可暂停并显示服务端错误", async () => {
    const active = agentStatus({ paused: false, phase: "deciding", llm_configured: true, bridge_connected: true });
    const api = client({
      getStatus: vi.fn().mockResolvedValue(active),
      pause: vi.fn().mockRejectedValue(new Error("暂停失败，请重试")),
    });
    render(<AgentPanel client={api} />);
    await screen.findByText("Agent 正在决策");

    fireEvent.click(screen.getByRole("button", { name: "暂停 Agent" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("暂停失败，请重试");
  });

  it("事件提交失败会显示原因并保留待发送内容", async () => {
    const api = client({
      submitEvents: vi.fn().mockRejectedValue(new Error("事件批次被拒绝")),
    });
    render(<AgentPanel client={api} />);
    await screen.findByText("Agent 已暂停");
    fireEvent.change(screen.getByLabelText("观众名称"), { target: { value: "小猫" } });
    fireEvent.change(screen.getByLabelText("聊天内容"), { target: { value: "请保留这条消息" } });

    fireEvent.click(screen.getByRole("button", { name: "发送模拟事件" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("事件批次被拒绝");
    expect(screen.getByLabelText("聊天内容")).toHaveValue("请保留这条消息");
  });
});

describe("Agent 轮询生命周期", () => {
  it("轮询串行执行且卸载会取消请求", async () => {
    vi.useFakeTimers();
    const pending = deferred<ReturnType<typeof agentStatus>>();
    let requestSignal: AbortSignal | undefined;
    const api = client({ getStatus: vi.fn().mockImplementation((signal) => { requestSignal = signal; return pending.promise; }) });
    const view = render(<AgentPanel client={api} pollIntervalMs={100} />);

    await act(async () => { await vi.advanceTimersByTimeAsync(1_000); });
    expect(api.getStatus).toHaveBeenCalledTimes(1);
    view.unmount();
    expect(requestSignal?.aborted).toBe(true);
    await act(async () => {
      pending.resolve(agentStatus());
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(api.getStatus).toHaveBeenCalledTimes(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("控制结果不会被先前 GET 的迟到响应覆盖", async () => {
    vi.useFakeTimers();
    const late = deferred<ReturnType<typeof agentStatus>>();
    const active = agentStatus({ paused: false, phase: "waiting", llm_configured: true, bridge_connected: true });
    const paused = agentStatus({ paused: true, phase: "paused", llm_configured: true, bridge_connected: true });
    const getStatus = vi.fn().mockResolvedValueOnce(active).mockImplementation(() => late.promise);
    const api = client({ getStatus, pause: vi.fn().mockResolvedValue(paused) });
    render(<AgentPanel client={api} pollIntervalMs={100} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    await act(async () => { await vi.advanceTimersByTimeAsync(100); });

    fireEvent.click(screen.getByRole("button", { name: "暂停 Agent" }));
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(screen.getByText("Agent 已暂停")).toBeVisible();

    await act(async () => {
      late.resolve(active);
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByText("Agent 已暂停")).toBeVisible();
  });

  it("卸载会取消正在执行的 Agent 动作", async () => {
    const pending = deferred<ReturnType<typeof agentStatus>>();
    let actionSignal: AbortSignal | undefined;
    const active = agentStatus({ paused: false, phase: "waiting", llm_configured: true, bridge_connected: true });
    const api = client({
      getStatus: vi.fn().mockResolvedValue(active),
      pause: vi.fn().mockImplementation((signal) => { actionSignal = signal; return pending.promise; }),
    });
    const view = render(<AgentPanel client={api} />);
    await screen.findByText("Agent 等待事件");
    fireEvent.click(screen.getByRole("button", { name: "暂停 Agent" }));

    view.unmount();

    expect(actionSignal?.aborted).toBe(true);
    pending.resolve(agentStatus());
  });
});
