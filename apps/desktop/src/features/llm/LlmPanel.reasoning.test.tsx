import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { LlmReasoningResult, LlmSettingsSnapshot } from "@meowlive/contracts";
import type { LlmClient } from "../../services/server/llm";
import { deferred } from "../../test/server-fixtures";
import { LlmPanel } from "./LlmPanel";

const settings = { provider: "openai", api_format: "openai_responses", base_url: "https://api.openai.com/v1", model: "gpt-5", mode: "cloud", timeout_seconds: 30, max_tokens: 4096, json_mode: true, reasoning_effort: "default" };
const snapshot: LlmSettingsSnapshot = { settings, key_configured: true, restart_required: false, active_model: "gpt-5", storage_available: true };
const mapped: LlmReasoningResult = { requested: "ultra", effective: "high", supported: ["minimal", "low", "medium", "high"], strategy: "openai_effort", budget_tokens: null, note: "已映射到模型最高档 high。", error: null };

function client(overrides: Partial<LlmClient> = {}): LlmClient {
  return {
    baseUrl: "test", getSettings: vi.fn().mockResolvedValue(snapshot),
    saveSettings: vi.fn().mockImplementation(async request => ({ ...snapshot, settings: request.settings, restart_required: true })),
    testSettings: vi.fn().mockResolvedValue({ message: "连接正常" }),
    listModels: vi.fn().mockResolvedValue({ base_url: settings.base_url, models: [{ id: "gpt-5", name: "GPT 5" }, { id: "other-model", name: "Other" }] }),
    previewReasoning: vi.fn().mockImplementation(async request => ({ ...mapped, requested: request.reasoning_effort, effective: request.reasoning_effort === "default" ? null : "high" })),
    ...overrides,
  };
}

async function load(api: LlmClient) {
  const view = render(<LlmPanel client={api} />);
  await screen.findByRole("option", { name: /gpt-5.*已保存/ });
  return view;
}

describe("LLM 推理强度", () => {
  it("显示八档、保留用户选择并将选择保存及重新加载", async () => {
    const api = client();
    const view = await load(api);
    const select = screen.getByRole("combobox", { name: "推理强度" });
    expect(within(select).getAllByRole("option").map(option => (option as HTMLOptionElement).value)).toEqual(["default", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"]);
    expect(select).toHaveValue("default");
    fireEvent.change(select, { target: { value: "ultra" } });
    expect(await screen.findByText("实际生效：high")).toBeVisible();
    expect(screen.getByText("支持档位：minimal / low / medium / high")).toBeVisible();
    expect(select).toHaveValue("ultra");
    expect(api.previewReasoning).toHaveBeenLastCalledWith({ provider: "openai", api_format: "openai_responses", model: "gpt-5", reasoning_effort: "ultra", max_tokens: 4096 }, expect.any(AbortSignal));
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));
    expect(await screen.findByRole("status")).toHaveTextContent("重启主服务");
    expect(api.saveSettings).toHaveBeenCalledWith(expect.objectContaining({ settings: expect.objectContaining({ reasoning_effort: "ultra" }) }), expect.any(AbortSignal));
    view.unmount();
    await load(client({ getSettings: vi.fn().mockResolvedValue({ ...snapshot, settings: { ...settings, reasoning_effort: "ultra" }, restart_required: true }) }));
    expect(screen.getByRole("combobox", { name: "推理强度" })).toHaveValue("ultra");
    expect(screen.getByRole("status")).toHaveTextContent("重启主服务");
  });

  it("只显示服务端的最低、最高和内部缺档映射", async () => {
    const api = client({ previewReasoning: vi.fn().mockImplementation(async request => ({ ...mapped, requested: request.reasoning_effort,
      supported: ["low", "high"], effective: request.reasoning_effort === "minimal" || request.reasoning_effort === "medium" ? "low" : "high" })) });
    await load(api);
    for (const [requested, effective] of [["minimal", "low"], ["ultra", "high"], ["medium", "low"]]) {
      fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: requested } });
      expect(await screen.findByText(`实际生效：${effective}`)).toBeVisible();
      expect(screen.getByLabelText("推理强度")).toHaveValue(requested);
    }
  });

  it("default 与未知模型明确不覆盖，并允许保存未知模型档位", async () => {
    const api = client({ previewReasoning: vi.fn().mockImplementation(async request => ({ ...mapped, requested: request.reasoning_effort,
      effective: request.reasoning_effort === "default" ? "default" : null, supported: [], note: request.reasoning_effort === "default" ? "保留供应商默认推理行为。" : "未识别该模型，不发送推理覆盖参数。" })) });
    await load(api);
    expect(await screen.findByText("实际生效：模型默认（不覆盖）")).toBeVisible();
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "ultra" } });
    expect(await screen.findByText("未识别该模型，不发送推理覆盖参数。")).toBeVisible();
    expect(screen.getByLabelText("推理强度")).toHaveValue("ultra");
    expect(screen.getByRole("button", { name: "保存配置" })).toBeEnabled();
  });

  it("预算错误阻止保存和测试，提高云端上限后显示可用预算", async () => {
    const api = client({ previewReasoning: vi.fn().mockImplementation(async request => ({ ...mapped, requested: request.reasoning_effort,
      budget_tokens: request.max_tokens >= 8192 ? 4096 : null, error: request.max_tokens >= 8192 ? null : "最大输出不足，请提高至至少 8192 tokens。" })) });
    await load(api);
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "high" } });
    expect(await screen.findByRole("alert")).toHaveTextContent("最大输出不足");
    const save = screen.getByRole("button", { name: "保存配置" });
    expect(save).toBeDisabled();
    expect(screen.getByRole("button", { name: "测试连接" })).toBeDisabled();
    fireEvent.submit(save.closest("form")!);
    expect(api.saveSettings).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("高级设置"));
    expect(screen.getByLabelText("最大输出（tokens）")).toHaveAttribute("max", "65536");
    fireEvent.change(screen.getByLabelText("最大输出（tokens）"), { target: { value: "8192" } });
    expect(await screen.findByText("推理预算：4096 tokens（包含在最大输出上限内）")).toBeVisible();
    expect(save).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "测试连接" }));
    await waitFor(() => expect(api.testSettings).toHaveBeenCalledWith(expect.objectContaining({ settings: expect.objectContaining({ max_tokens: 8192, reasoning_effort: "high" }) }), expect.any(AbortSignal)));
    await screen.findByText("连接正常");
    fireEvent.change(screen.getByLabelText("运行模式"), { target: { value: "local" } });
    expect(screen.getByLabelText("最大输出（tokens）")).toHaveAttribute("max", "1024");
  });

  it("切换模型取消旧预览并忽略迟到响应，卸载取消当前请求", async () => {
    const late = deferred<LlmReasoningResult>();
    const signals: AbortSignal[] = [];
    const api = client({ previewReasoning: vi.fn().mockImplementation((_request, signal) => { signals.push(signal); return signals.length === 1 ? late.promise : Promise.resolve({ ...mapped, requested: "ultra", effective: "low", supported: ["low", "high"] }); }) });
    const view = await load(api);
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "ultra" } });
    await waitFor(() => expect(signals).toHaveLength(1));
    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    await screen.findByRole("option", { name: "Other（other-model）" });
    fireEvent.change(screen.getByLabelText("模型"), { target: { value: "other-model" } });
    expect(signals[0].aborted).toBe(true);
    expect(await screen.findByText("实际生效：low")).toBeVisible();
    await act(async () => late.resolve(mapped));
    expect(screen.queryByText("实际生效：high")).not.toBeInTheDocument();
    expect(screen.getByLabelText("推理强度")).toHaveValue("ultra");
    vi.mocked(api.previewReasoning).mockImplementation((_request, signal) => { signals.push(signal!); return new Promise(() => {}); });
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "max" } });
    await waitFor(() => expect(signals).toHaveLength(3));
    view.unmount();
    expect(signals[2].aborted).toBe(true);
  });

  it("合并快速输入且无关字段变更不重复预览，失败可重试", async () => {
    const api = client({ previewReasoning: vi.fn().mockRejectedValueOnce(new Error("预览暂不可用")).mockResolvedValue(mapped) });
    await load(api);
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "low" } });
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "medium" } });
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "ultra" } });
    expect(await screen.findByText("预览暂不可用")).toBeVisible();
    expect(api.previewReasoning).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "保存配置" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "重新预览" }));
    expect(await screen.findByText("实际生效：high")).toBeVisible();
    fireEvent.click(screen.getByText("高级设置"));
    fireEvent.change(screen.getByLabelText("超时时间（秒）"), { target: { value: "60" } });
    await act(async () => { await new Promise(resolve => setTimeout(resolve, 250)); });
    expect(api.previewReasoning).toHaveBeenCalledTimes(2);
  });

  it("更改 API 格式和服务商后清空旧映射，并按新连接重新预览", async () => {
    const api = client({ getSettings: vi.fn().mockResolvedValue({ ...snapshot, key_configured: false }),
      listModels: vi.fn().mockImplementation(async request => ({ base_url: request.base_url, models: [{ id: "chosen-model", name: "Chosen" }] })) });
    await load(api);
    fireEvent.change(screen.getByLabelText("推理强度"), { target: { value: "ultra" } });
    await screen.findByText("实际生效：high");
    fireEvent.click(screen.getByText("高级设置"));
    fireEvent.change(screen.getByLabelText("API 格式"), { target: { value: "openai_chat" } });
    expect(screen.queryByText("实际生效：high")).not.toBeInTheDocument();
    expect(screen.getByLabelText("模型")).toHaveValue("");
    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    await screen.findByRole("option", { name: "Chosen（chosen-model）" });
    fireEvent.change(screen.getByLabelText("模型"), { target: { value: "chosen-model" } });
    await screen.findByText("实际生效：high");
    expect(api.previewReasoning).toHaveBeenLastCalledWith(expect.objectContaining({ provider: "openai", api_format: "openai_chat", model: "chosen-model", reasoning_effort: "ultra" }), expect.any(AbortSignal));
    fireEvent.change(screen.getByLabelText("服务商"), { target: { value: "claude" } });
    expect(screen.queryByText("实际生效：high")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "获取模型" }));
    await screen.findByRole("option", { name: "Chosen（chosen-model）" });
    fireEvent.change(screen.getByLabelText("模型"), { target: { value: "chosen-model" } });
    await screen.findByText("实际生效：high");
    expect(api.previewReasoning).toHaveBeenLastCalledWith(expect.objectContaining({ provider: "claude", api_format: "anthropic_messages", model: "chosen-model", reasoning_effort: "ultra" }), expect.any(AbortSignal));
  });

  it("default 的预览失败不阻止原有连接测试，空模型继续显示原有校验", async () => {
    const api = client({ previewReasoning: vi.fn().mockRejectedValue(new Error("预览暂不可用")) });
    await load(api);
    await screen.findByText("预览暂不可用");
    expect(screen.getByRole("button", { name: "保存配置" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "测试连接" }));
    await screen.findByText("连接正常");
    expect(api.testSettings).toHaveBeenCalledWith(expect.objectContaining({ settings: expect.objectContaining({ reasoning_effort: "default" }) }), expect.any(AbortSignal));
    fireEvent.change(screen.getByLabelText("模型"), { target: { value: "" } });
    expect(screen.queryByText("预览暂不可用")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("选择要使用的模型");
    expect(api.previewReasoning).toHaveBeenCalledTimes(1);
    expect(api.saveSettings).not.toHaveBeenCalled();
  });
});
