import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { createAgentClient } from "../../services/server/agent";
import { createLlmClient } from "../../services/server/llm";
import { createLlmRuntimeClient } from "../../services/server/llm-runtime";
import { agentStatus } from "../../test/agent-fixtures";
import { AgentPanel } from "../agent/AgentPanel";
import { LlmPanel } from "../llm/LlmPanel";
import { activitySnapshot, runtimeSnapshot } from "./runtime-fixtures";

describe("运行层页面接入", () => {
  it("LLM 页用已保存连接填入价格身份，不读取密钥", async () => {
    const llm = { settings: { provider: "custom", api_format: "openai_chat", base_url: "https://llm.example.com/v1", model: "chosen-model", mode: "cloud", timeout_seconds: 30, max_tokens: 1024, json_mode: true, reasoning_effort: "default" },
      key_configured: true, restart_required: false, active_model: "chosen-model", storage_available: true };
    const client = createLlmClient({ fetcher: async url => new Response(JSON.stringify(String(url).endsWith("/reasoning")
      ? { requested: "default", effective: null, supported: [], strategy: "unsupported", budget_tokens: null, note: "保留模型默认行为。", error: null } : llm)) });
    let savedPrice: Record<string, unknown> | undefined;
    const runtime = createLlmRuntimeClient({ fetcher: async (_url, init) => {
      if (init?.body) savedPrice = JSON.parse(String(init.body)).settings.prices[0] as Record<string, unknown>;
      return new Response(JSON.stringify(runtimeSnapshot()));
    } });
    render(<LlmPanel client={client} runtimeClient={runtime} />);
    await screen.findByRole("option", { name: /chosen-model/ });
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await screen.findByLabelText("搜索地址");
    fireEvent.click(screen.getByText("模型价格"));
    fireEvent.click(screen.getByRole("button", { name: "添加当前连接价格" }));
    expect(screen.getByLabelText("价格模型 1")).toHaveValue("chosen-model");
    expect(screen.getByLabelText("价格 API 地址 1")).toHaveValue("https://llm.example.com/v1");
    expect(screen.getByLabelText("API 密钥")).toHaveValue("");
    for (const label of ["输入单价", "输出单价", "缓存读取单价", "缓存写入单价"]) fireEvent.change(screen.getByLabelText(`${label} 1`), { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "保存运行配置" }));
    await screen.findByText("运行配置已保存并生效。");
    expect(savedPrice).toEqual({ provider: "custom", base_url: "https://llm.example.com/v1", model: "chosen-model",
      input_usd_per_million: 2, output_usd_per_million: 2, cache_read_usd_per_million: 2, cache_write_usd_per_million: 2 });
  });

  it("Agent 互动页直接展示当前工具进度", async () => {
    const client = createAgentClient({ fetcher: async () => new Response(JSON.stringify(agentStatus())) });
    const runtime = createLlmRuntimeClient({ fetcher: async () => new Response(JSON.stringify(activitySnapshot())) });
    render(<AgentPanel client={client} runtimeClient={runtime} />);
    expect(await screen.findByText("正在查资料")).toBeVisible();
    expect(screen.getByText("网页搜索")).toBeVisible();
  });
});
