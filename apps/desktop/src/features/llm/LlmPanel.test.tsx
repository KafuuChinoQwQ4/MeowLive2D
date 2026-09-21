import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { LlmSettingsSnapshot } from "@meowlive/contracts";
import type { LlmClient } from "../../services/server/llm";
import { deferred } from "../../test/server-fixtures";
import { LlmPanel } from "./LlmPanel";

const snapshot = (overrides: Partial<LlmSettingsSnapshot> = {}): LlmSettingsSnapshot => ({
  settings: {
    provider: "openai", api_format: "openai_responses", base_url: "https://api.openai.com/v1",
    model: "gpt-4o-mini", mode: "cloud", timeout_seconds: 30, max_tokens: 1024, json_mode: true, reasoning_effort: "default",
  },
  key_configured: true, restart_required: false, active_model: "gpt-4o-mini", storage_available: true,
  ...overrides,
});

function client(overrides: Partial<LlmClient> = {}): LlmClient {
  return {
    baseUrl: "http://127.0.0.1:19600",
    getSettings: vi.fn().mockResolvedValue(snapshot()),
    saveSettings: vi.fn().mockImplementation(async request => snapshot({ settings: request.settings, restart_required: true })),
    testSettings: vi.fn().mockResolvedValue({ message: "连接测试成功" }),
    listModels: vi.fn().mockResolvedValue({ base_url: "https://api.openai.com/v1", models: [
      { id: "gpt-4o-mini", name: "GPT 4o Mini" }, { id: "gpt-4.1-mini", name: "GPT 4.1 Mini" },
    ] }),
    previewReasoning: vi.fn().mockImplementation(async request => ({ requested: request.reasoning_effort, effective: null, supported: [], strategy: "unsupported", budget_tokens: null, note: "保留模型默认行为。", error: null })),
    ...overrides,
  };
}

describe("LLM 接入面板", () => {
  it("加载时不回填密钥，测试使用当前草稿且不自动保存", async () => {
    const api = client();
    render(<LlmPanel client={api} />);
    await screen.findByRole("option", { name: /gpt-4o-mini.*已保存/ });
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("gpt-4o-mini");
    expect(screen.getByLabelText("API 密钥")).toHaveValue("");
    expect(screen.getByLabelText("API 密钥")).toHaveAttribute("placeholder", "已安全保存；留空则继续使用");

    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "模型" }), "gpt-4.1-mini");
    await userEvent.click(screen.getByRole("button", { name: "测试连接" }));

    expect(api.testSettings).toHaveBeenCalledWith(expect.objectContaining({
      settings: expect.objectContaining({ model: "gpt-4.1-mini" }), api_key: null, clear_api_key: false,
    }), expect.any(AbortSignal));
    expect(api.saveSettings).not.toHaveBeenCalled();
    expect(await screen.findByRole("status")).toHaveTextContent("连接测试成功");
    expect(screen.getByText(/少量调用费用/)).toBeVisible();
  });

  it("切换供应商套用协议和地址、清空模型，并要求明确处理密钥", async () => {
    const api = client();
    render(<LlmPanel client={api} />);
    await screen.findByRole("option", { name: /gpt-4o-mini.*已保存/ });
    await userEvent.click(screen.getByText("高级设置"));

    await userEvent.selectOptions(screen.getByRole("combobox", { name: "服务商" }), "claude");
    expect(screen.getByRole("combobox", { name: "API 格式" })).toHaveValue("anthropic_messages");
    expect(screen.getByRole("textbox", { name: "API 地址" })).toHaveValue("https://api.anthropic.com/v1");
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("");
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("新的 API 密钥或明确移除");
    expect(api.listModels).not.toHaveBeenCalled();

    vi.mocked(api.listModels).mockResolvedValue({ base_url: "https://api.anthropic.com/v1", models: [{ id: "claude-model", name: "Claude Model" }] });
    await userEvent.click(screen.getByRole("checkbox", { name: "移除已保存密钥" }));
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "模型" }), "claude-model");
    await userEvent.click(screen.getByRole("button", { name: "保存配置" }));
    expect(api.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ api_key: null, clear_api_key: true }), expect.any(AbortSignal));
    expect(await screen.findByRole("status")).toHaveTextContent("重启主服务");
    expect(screen.getByRole("link", { name: "前往运行总览" })).toHaveAttribute("href", "#overview");
  });

  it("OpenAI 预设使用 Responses API 且自定义配置可选择全部格式", async () => {
    render(<LlmPanel client={client()} />);
    await screen.findByRole("option", { name: /gpt-4o-mini.*已保存/ });
    expect(screen.getByLabelText("API 格式")).not.toBeVisible();
    await userEvent.click(screen.getByText("高级设置"));
    expect(screen.getByRole("combobox", { name: "API 格式" })).toHaveValue("openai_responses");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "服务商" }), "custom");
    expect(screen.getByRole("combobox", { name: "API 格式" })).toContainHTML("OpenAI Responses");
  });

  it("没有配置存储时只能测试，并说明应改用项目启动入口", async () => {
    const api = client({ getSettings: vi.fn().mockResolvedValue(snapshot({ storage_available: false })) });
    render(<LlmPanel client={api} />);
    expect(await screen.findByText("此主服务未启用配置保存，可测试连接；请使用项目主服务启动入口后保存。")).toBeVisible();
    const save = screen.getByRole("button", { name: "保存配置" });
    expect(save).toBeDisabled();
    expect(screen.getByRole("button", { name: "测试连接" })).toBeEnabled();
    fireEvent.submit(save.closest("form")!);
    expect(api.saveSettings).not.toHaveBeenCalled();
  });

  it("初次加载待重启配置时立即显示提示和运行总览链接", async () => {
    render(<LlmPanel client={client({ getSettings: vi.fn().mockResolvedValue(snapshot({ restart_required: true })) })} />);
    expect(await screen.findByRole("status")).toHaveTextContent("重启主服务");
    expect(screen.getByRole("link", { name: "前往运行总览" })).toHaveAttribute("href", "#overview");
  });

  it("忽略已取消加载的迟到响应，不覆盖新 client 的草稿", async () => {
    const late = deferred<LlmSettingsSnapshot>();
    const first = client({ getSettings: vi.fn().mockReturnValue(late.promise) });
    const current = snapshot({ settings: { ...snapshot().settings, model: "current-model" } });
    const second = client({ getSettings: vi.fn().mockResolvedValue(current) });
    const view = render(<LlmPanel client={first} />);
    view.rerender(<LlmPanel client={second} />);
    expect(await screen.findByRole("option", { name: /current-model.*已保存/ })).toBeInTheDocument();
    await act(async () => late.resolve(snapshot({ settings: { ...snapshot().settings, model: "stale-model" } })));
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("current-model");
  });

  it("切换 client 会取消旧操作并忽略迟到的测试结果", async () => {
    const late = deferred<{ message: string }>();
    let actionSignal: AbortSignal | undefined;
    const first = client({ testSettings: vi.fn().mockImplementation((_request, signal) => { actionSignal = signal; return late.promise; }) });
    const second = client({ getSettings: vi.fn().mockResolvedValue(snapshot({ settings: { ...snapshot().settings, model: "new-service-model" } })) });
    const view = render(<LlmPanel client={first} />);
    await screen.findByRole("option", { name: /gpt-4o-mini.*已保存/ });
    await userEvent.click(screen.getByRole("button", { name: "测试连接" }));
    view.rerender(<LlmPanel client={second} />);
    expect(await screen.findByRole("option", { name: /new-service-model.*已保存/ })).toBeInTheDocument();
    expect(actionSignal?.aborted).toBe(true);
    await act(async () => late.resolve({ message: "旧服务测试成功" }));
    expect(screen.queryByText("旧服务测试成功")).not.toBeInTheDocument();
  });

  it("加载失败可重试，卸载取消请求，操作期间禁用控件", async () => {
    const load = deferred<LlmSettingsSnapshot>();
    let signal: AbortSignal | undefined;
    const api = client({ getSettings: vi.fn().mockImplementation(current => { signal = current; return load.promise; }) });
    const view = render(<LlmPanel client={api} />);
    expect(screen.getByRole("button", { name: "保存配置" })).toBeDisabled();
    view.unmount();
    expect(signal?.aborted).toBe(true);
    await act(async () => load.resolve(snapshot()));

    const retryApi = client({ getSettings: vi.fn().mockRejectedValueOnce(new Error("读取失败")).mockResolvedValueOnce(snapshot()) });
    render(<LlmPanel client={retryApi} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("读取失败");
    fireEvent.click(screen.getByRole("button", { name: "重新加载" }));
    expect(await screen.findByRole("option", { name: /gpt-4o-mini.*已保存/ })).toBeInTheDocument();
  });
});
