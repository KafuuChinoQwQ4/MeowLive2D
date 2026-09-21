import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { LlmModelsResult, LlmSettingsSnapshot } from "@meowlive/contracts";
import { createLlmClient, type LlmClient } from "../../services/server/llm";
import { deferred } from "../../test/server-fixtures";
import { LlmPanel } from "./LlmPanel";

const initial: LlmSettingsSnapshot = {
  settings: { provider: "custom", api_format: "openai_chat", base_url: "", model: "", mode: "cloud", timeout_seconds: 30, max_tokens: 1024, json_mode: true, reasoning_effort: "default" },
  key_configured: false, restart_required: false, active_model: "", storage_available: true,
};
const catalog = { base_url: "https://example.com/v1", models: [{ id: "actual-model", name: "Actual Model" }, { id: "second-model", name: "Second Model" }] };
const saved: LlmSettingsSnapshot = {
  ...initial, key_configured: true,
  settings: { ...initial.settings, base_url: "https://example.com/v1", model: "actual-model" },
};
function client(snapshot = initial, overrides: Partial<LlmClient> = {}): LlmClient {
  return {
    baseUrl: "http://127.0.0.1:19600", getSettings: vi.fn().mockResolvedValue(snapshot),
    listModels: vi.fn().mockResolvedValue(catalog), testSettings: vi.fn().mockResolvedValue({ message: "连接正常" }),
    saveSettings: vi.fn().mockImplementation(async request => ({ ...snapshot, settings: request.settings, restart_required: true })),
    previewReasoning: vi.fn().mockImplementation(async request => ({ requested: request.reasoning_effort, effective: null, supported: [], strategy: "unsupported", budget_tokens: null, note: "保留模型默认行为。", error: null })),
    ...overrides,
  };
}
async function loaded(api: LlmClient) {
  const view = render(<LlmPanel client={api} />);
  await waitFor(() => expect(screen.getByLabelText("API 地址")).toBeEnabled());
  return view;
}

describe("LLM 模型选择流程", () => {
  it("空配置只填地址和密钥即可获取真实模型，选择后测试和保存同一草稿", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const api = createLlmClient({ fetcher: async (url, init) => {
      const body = init?.body ? JSON.parse(String(init.body)) : undefined;
      calls.push({ path: String(url), body });
      if (String(url).endsWith("/reasoning")) return new Response(JSON.stringify({ requested: body.reasoning_effort, effective: null, supported: [], strategy: "unsupported", budget_tokens: null, note: "保留模型默认行为。", error: null }));
      if (String(url).endsWith("/models")) return new Response(JSON.stringify(catalog));
      if (String(url).endsWith("/test")) return new Response(JSON.stringify({ message: "连接正常" }));
      return new Response(JSON.stringify(body ? { ...initial, settings: body.settings, restart_required: true } : initial));
    } });
    await loaded(api);
    expect(screen.queryByRole("textbox", { name: /模型/ })).not.toBeInTheDocument();
    expect(screen.getByLabelText("API 格式")).not.toBeVisible();
    fireEvent.change(screen.getByLabelText("API 地址"), { target: { value: "https://example.com" } });
    fireEvent.change(screen.getByLabelText("API 密钥"), { target: { value: "draft-key" } });
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByText("已获取 2 个模型。" )).toBeVisible();
    expect(calls[1]).toEqual({ path: "http://127.0.0.1:19600/api/llm/models", body: {
      provider: "custom", api_format: "openai_chat", base_url: "https://example.com", mode: "cloud", api_key: "draft-key", clear_api_key: false,
    } });
    expect(calls).toHaveLength(2);
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "模型" }), "second-model");
    await userEvent.click(screen.getByRole("button", { name: "测试连接" }));
    expect(await screen.findByText("连接正常")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "保存配置" }));
    expect(await screen.findByRole("status")).toHaveTextContent("重启主服务");
    const expected = { settings: { ...initial.settings, base_url: "https://example.com/v1", model: "second-model" }, api_key: "draft-key", clear_api_key: false };
    expect(calls.find(call => call.path.endsWith("/test"))).toEqual({ path: "http://127.0.0.1:19600/api/llm/test", body: expected });
    expect(calls.find(call => call.path.endsWith("/settings") && call.body)).toEqual({ path: "http://127.0.0.1:19600/api/llm/settings", body: expected });
  });

  it.each([
    ["https://api.openai.com", "openai", "openai_responses"],
    ["https://api.anthropic.com", "claude", "anthropic_messages"],
    ["https://generativelanguage.googleapis.com", "gemini", "gemini_generate_content"],
    ["https://api.moonshot.cn/v1", "kimi", "openai_chat"],
    ["https://api.moonshot.ai/v1", "kimi", "openai_chat"],
    ["https://open.bigmodel.cn/api/paas/v4", "glm", "openai_chat"],
    ["https://api.x.ai/v1", "grok", "openai_chat"],
    ["https://api.deepseek.com", "deepseek", "openai_chat"],
    ["https://gateway.example/v1/responses", "custom", "openai_responses"],
    ["https://gateway.example/v1/messages", "custom", "anthropic_messages"],
    ["https://gateway.example/v1beta/models/gemini-flash:generateContent", "custom", "gemini_generate_content"],
    ["https://api.openai.com.evil.example/v1", "custom", "openai_chat"],
  ])("根据地址 %s 推断供应商和协议", async (url, provider, apiFormat) => {
    const api = client();
    await loaded(api);
    fireEvent.change(screen.getByLabelText("API 地址"), { target: { value: url } });
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(api.listModels).toHaveBeenCalledWith(expect.objectContaining({ provider, api_format: apiFormat, base_url: url }), expect.any(AbortSignal));
  });

  it("手动 API 格式保持到用户再次更改地址，供应商切换保留用户填写的密钥", async () => {
    const api = client();
    await loaded(api);
    fireEvent.change(screen.getByLabelText("API 地址"), { target: { value: "https://api.openai.com" } });
    await userEvent.click(screen.getByText("高级设置"));
    await userEvent.selectOptions(screen.getByLabelText("API 格式"), "anthropic_messages");
    fireEvent.change(screen.getByLabelText("API 密钥"), { target: { value: "draft-key" } });
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(api.listModels).toHaveBeenLastCalledWith(expect.objectContaining({ api_format: "anthropic_messages", api_key: "draft-key" }), expect.any(AbortSignal));
    fireEvent.change(screen.getByLabelText("API 地址"), { target: { value: "https://unknown.example" } });
    expect(screen.getByLabelText("API 格式")).toHaveValue("openai_chat");
    expect(screen.getByLabelText("服务商")).toHaveValue("custom");
    await userEvent.selectOptions(screen.getByLabelText("服务商"), "gemini");
    expect(screen.getByLabelText("API 密钥")).toHaveValue("draft-key");
  });

  it("已保存模型无需重取即可测试，显式获取后只保留列表仍有的选择", async () => {
    const api = client(saved);
    await loaded(api);
    expect(screen.getByRole("option", { name: /actual-model.*已保存.*尚未验证/ })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "测试连接" }));
    expect(api.testSettings).toHaveBeenCalledWith(expect.objectContaining({ api_key: null }), expect.any(AbortSignal));
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("actual-model");
    expect(screen.queryByRole("option", { name: /尚未验证/ })).not.toBeInTheDocument();
    vi.mocked(api.listModels).mockResolvedValue({ ...catalog, models: [{ id: "replacement", name: "Replacement" }] });
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("");
    expect(screen.queryByRole("option", { name: /actual-model/ })).not.toBeInTheDocument();
  });

  it("相同连接的规范化地址仍可复用保存密钥，其他源绝不发送保存密钥", async () => {
    const api = client({ ...saved, settings: { ...saved.settings, base_url: "https://example.com/responses" } }, {
      listModels: vi.fn().mockResolvedValue({ ...catalog, base_url: "https://example.com" }),
    });
    await loaded(api);
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(screen.getByLabelText("API 地址")).toHaveValue("https://example.com");
    await userEvent.click(screen.getByRole("button", { name: "测试连接" }));
    expect(api.testSettings).toHaveBeenCalledWith(expect.objectContaining({ api_key: null, settings: expect.objectContaining({ base_url: "https://example.com" }) }), expect.any(AbortSignal));
    fireEvent.change(screen.getByLabelText("API 地址"), { target: { value: "https://other.example/v1" } });
    expect(screen.getByLabelText("API 密钥")).not.toHaveAttribute("placeholder", "已安全保存；留空则继续使用");
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("新的 API 密钥或明确移除");
    expect(api.listModels).toHaveBeenCalledTimes(1);
  });

  it("已保存版本路径的密钥不能用于同源根地址", async () => {
    const api = client(saved);
    await loaded(api);
    fireEvent.change(screen.getByLabelText("API 地址"), { target: { value: "https://example.com" } });
    expect(screen.getByLabelText("API 密钥")).not.toHaveAttribute("placeholder", "已安全保存；留空则继续使用");
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("新的 API 密钥或明确移除");
    expect(api.listModels).not.toHaveBeenCalled();
  });

  it.each(["API 地址", "API 密钥", "API 格式", "运行模式", "移除已保存密钥"])("更改 %s 后清空旧模型及选择", async label => {
    const api = client(saved);
    await loaded(api);
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    await userEvent.click(screen.getByText("高级设置"));
    if (label === "移除已保存密钥") await userEvent.click(screen.getByLabelText(label));
    else fireEvent.change(screen.getByLabelText(label), { target: { value: {
      "API 地址": "https://other.example", "API 密钥": "new-key", "API 格式": "anthropic_messages", "运行模式": "local",
    }[label] } });
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("");
    expect(screen.queryByRole("option", { name: "Actual Model（actual-model）" })).not.toBeInTheDocument();
    expect(screen.queryByText("已获取 2 个模型。")).not.toBeInTheDocument();
  });

  it("空列表和服务商失败提供可重试反馈且不生成模型选项", async () => {
    const api = client(saved, { listModels: vi.fn().mockResolvedValueOnce({ ...catalog, models: [] }).mockRejectedValueOnce(new Error("供应商拒绝了密钥")).mockResolvedValueOnce(catalog) });
    await loaded(api);
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByText(/未返回可选模型/)).toBeVisible();
    expect(screen.getByRole("combobox", { name: "模型" })).toHaveValue("");
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("供应商拒绝了密钥");
    expect(within(screen.getByRole("combobox", { name: "模型" })).getAllByRole("option")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    expect(await screen.findByText("已获取 2 个模型。")).toBeVisible();
  });

  it("更改连接会取消获取并忽略迟到列表，卸载也取消请求", async () => {
    const late = deferred<LlmModelsResult>();
    let signal: AbortSignal | undefined;
    const api = client(saved, { listModels: vi.fn().mockImplementationOnce((_request, current) => { signal = current; return late.promise; }).mockResolvedValue(catalog) });
    const view = await loaded(api);
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    fireEvent.change(screen.getByLabelText("API 密钥"), { target: { value: "new-key" } });
    expect(signal?.aborted).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    await act(async () => late.resolve({ ...catalog, models: [{ id: "stale-model", name: "Stale Model" }] }));
    expect(screen.queryByRole("option", { name: /Stale/ })).not.toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Actual Model（actual-model）" })).toBeInTheDocument();
    vi.mocked(api.listModels).mockImplementationOnce((_request, current) => { signal = current; return new Promise(() => {}); });
    await userEvent.click(screen.getByRole("button", { name: "获取模型" }));
    view.unmount();
    expect(signal?.aborted).toBe(true);
  });
});
