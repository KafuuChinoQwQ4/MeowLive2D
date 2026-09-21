import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { AgentRuntimeSettingsRequest } from "@meowlive/contracts";
import { createLlmRuntimeClient } from "../../services/server/llm-runtime";
import { runtimeSnapshot, usageSnapshot } from "./runtime-fixtures";
import { LlmRuntimePanel } from "./LlmRuntimePanel";

const connection = { provider: "openai", base_url: "https://api.openai.com/v1", model: "chat-model" };

describe("运行配置与用量", () => {
  it("保留空白密钥、保存开关和价格后清空密码输入并确认生效", async () => {
    const writes: AgentRuntimeSettingsRequest[] = [];
    const client = createLlmRuntimeClient({ fetcher: async (_url, init) => {
      if (init?.method === "POST") writes.push(JSON.parse(String(init.body)) as AgentRuntimeSettingsRequest);
      return new Response(JSON.stringify({ ...runtimeSnapshot(), settings: writes.at(-1)?.settings ?? runtimeSnapshot().settings }));
    } });
    render(<LlmRuntimePanel client={client} connection={connection} />);
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await screen.findByLabelText("搜索 API 密钥");
    expect(screen.getByLabelText("搜索 API 密钥")).toHaveValue("");
    fireEvent.click(screen.getByRole("checkbox", { name: "流式接收" }));
    fireEvent.click(screen.getByText("模型价格"));
    fireEvent.click(screen.getByRole("button", { name: "添加当前连接价格" }));
    fireEvent.change(screen.getByLabelText("输入单价 1"), { target: { value: "2.5" } });
    fireEvent.change(screen.getByLabelText("输出单价 1"), { target: { value: "5" } });
    fireEvent.change(screen.getByLabelText("缓存读取单价 1"), { target: { value: "0.25" } });
    fireEvent.change(screen.getByLabelText("缓存写入单价 1"), { target: { value: "3" } });
    fireEvent.click(screen.getByRole("button", { name: "保存运行配置" }));
    expect(await screen.findByRole("status")).toHaveTextContent("已保存并生效");
    expect(writes[0]).toMatchObject({ search_api_key: null, clear_search_api_key: false, settings: { streaming: false,
      prices: [{ ...connection, input_usd_per_million: 2.5, output_usd_per_million: 5, cache_read_usd_per_million: 0.25, cache_write_usd_per_million: 3 }] } });
    fireEvent.change(screen.getByLabelText("搜索 API 密钥"), { target: { value: "new-private-key" } });
    fireEvent.click(screen.getByRole("button", { name: "保存运行配置" }));
    await act(async () => {});
    expect(writes[1].search_api_key).toBe("new-private-key");
    expect(screen.getByLabelText("搜索 API 密钥")).toHaveValue("");
    expect(screen.queryByText("new-private-key")).not.toBeInTheDocument();
  });

  it("搜索身份变化后不复用旧密钥，明确清除后才允许保存", async () => {
    const writes: unknown[] = [];
    const client = createLlmRuntimeClient({ fetcher: async (_url, init) => {
      if (init?.body) writes.push(JSON.parse(String(init.body)));
      return new Response(JSON.stringify(runtimeSnapshot()));
    } });
    render(<LlmRuntimePanel client={client} connection={connection} />);
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await screen.findByLabelText("搜索地址");
    fireEvent.change(screen.getByLabelText("搜索地址"), { target: { value: "https://api.search.brave.com/res/v1/web/search/" } });
    fireEvent.click(screen.getByRole("button", { name: "保存运行配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("填写新的搜索密钥或明确移除");
    expect(writes).toHaveLength(0);
    fireEvent.click(screen.getByRole("checkbox", { name: "移除已保存搜索密钥" }));
    fireEvent.click(screen.getByRole("button", { name: "保存运行配置" }));
    await screen.findByRole("status");
    expect(writes[0]).toMatchObject({ clear_search_api_key: true, search_api_key: null });
  });

  it("展示缓存和推理子集、部分估算及未知费用，并将日期筛选发送到服务", async () => {
    const urls: string[] = [];
    const client = createLlmRuntimeClient({ fetcher: async url => { urls.push(String(url)); return new Response(JSON.stringify(String(url).includes("/usage") ? usageSnapshot() : runtimeSnapshot())); } });
    render(<LlmRuntimePanel client={client} connection={connection} />);
    fireEvent.click(screen.getByText("用量与费用"));
    expect(await screen.findByText("已计价部分 $0.012500 USD")).toBeVisible();
    expect(screen.getByText(/1 次调用未报告用量/)).toBeVisible();
    const totals = screen.getByRole("group", { name: "用量汇总" });
    expect(within(totals).getByText("1,000")).toBeVisible();
    expect(within(totals).getByText("50")).toBeVisible();
    fireEvent.click(screen.getByText("最近调用记录"));
    const record = screen.getByRole("list", { name: "调用记录" });
    expect(within(record).getByText("未计价 / 未报告用量")).toBeVisible();
    expect(within(record).queryByText(/\$0\.000000/)).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("起始日期"), { target: { value: "2026-09-01" } });
    fireEvent.change(screen.getByLabelText("结束日期"), { target: { value: "2026-09-22" } });
    fireEvent.change(screen.getByLabelText("筛选模型"), { target: { value: "chat-model" } });
    fireEvent.click(screen.getByRole("button", { name: "查询用量" }));
    await act(async () => {});
    const query = new URL(urls.at(-1)!).searchParams;
    expect(Number(query.get("since_ms"))).toBe(new Date(2026, 8, 1).getTime());
    expect(Number(query.get("until_ms"))).toBe(new Date(2026, 8, 23).getTime() - 1);
    expect(query.get("model")).toBe("chat-model");
  });
  it("只有部分 token 已报告时保留已知总数，整轮未计价不显示零费用", async () => {
    const snapshot = usageSnapshot();
    snapshot.totals = { ...snapshot.totals, calls: 1, input_tokens: 1200, output_tokens: 0, unpriced_calls: 1, unknown_usage_calls: 1, estimated_cost_microusd: 0 };
    snapshot.groups = [];
    const client = createLlmRuntimeClient({ fetcher: async () => new Response(JSON.stringify(snapshot)) });
    render(<LlmRuntimePanel client={client} connection={connection} />);
    fireEvent.click(screen.getByText("用量与费用"));
    const summary = await screen.findByRole("group", { name: "用量汇总" });
    expect(within(summary).getByText("1,200")).toBeVisible();
    expect(within(summary).getByText("未计价 / 未报告用量")).toBeVisible();
    expect(within(summary).queryByText(/\$0\.000000/)).not.toBeInTheDocument();
  });

  it("隐藏页面取消保存等待，迟到响应不清除草稿或显示成功", async () => {
    let saveSignal: AbortSignal | null | undefined;
    let finish!: (value: Response) => void;
    const pending = new Promise<Response>(resolve => { finish = resolve; });
    const client = createLlmRuntimeClient({ fetcher: async (_url, init) => {
      if (init?.method === "POST") { saveSignal = init.signal; return pending; }
      return new Response(JSON.stringify(runtimeSnapshot()));
    } });
    const panel = <LlmRuntimePanel client={client} connection={connection} />;
    const view = render(<section>{panel}</section>);
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await screen.findByLabelText("搜索地址");
    fireEvent.click(screen.getByRole("checkbox", { name: "流式接收" }));
    fireEvent.click(screen.getByRole("button", { name: "保存运行配置" }));
    view.rerender(<section hidden>{panel}</section>);
    await act(async () => {});
    expect(saveSignal?.aborted).toBe(true);
    await act(async () => { finish(new Response(JSON.stringify(runtimeSnapshot()))); });
    view.rerender(<section>{panel}</section>);
    await act(async () => {});
    expect(screen.getByRole("checkbox", { name: "流式接收" })).not.toBeChecked();
    expect(screen.queryByText("运行配置已保存并生效。")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存运行配置" })).toBeEnabled();
  });
  it("切换主服务取消旧保存并恢复新配置的可编辑状态", async () => {
    let previousSignal: AbortSignal | null | undefined;
    const first = createLlmRuntimeClient({ fetcher: async (_url, init) => {
      if (init?.method === "POST") { previousSignal = init.signal; return new Promise<Response>(() => {}); }
      return new Response(JSON.stringify(runtimeSnapshot()));
    } });
    const second = createLlmRuntimeClient({ fetcher: async () => new Response(JSON.stringify(runtimeSnapshot())) });
    const view = render(<LlmRuntimePanel client={first} connection={connection} />);
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await screen.findByLabelText("搜索地址");
    fireEvent.click(screen.getByRole("button", { name: "保存运行配置" }));
    view.rerender(<LlmRuntimePanel client={second} connection={connection} />);
    await act(async () => {});
    expect(previousSignal?.aborted).toBe(true);
    expect(screen.getByRole("checkbox", { name: "流式接收" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "保存运行配置" })).toBeEnabled();
  });
  it("折叠运行设置保留未保存修改，再展开不会回退到旧配置", async () => {
    const client = createLlmRuntimeClient({ fetcher: async () => new Response(JSON.stringify(runtimeSnapshot())) });
    render(<LlmRuntimePanel client={client} connection={connection} />);
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await screen.findByLabelText("搜索地址");
    fireEvent.click(screen.getByRole("checkbox", { name: "流式接收" }));
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await act(async () => { await new Promise(resolve => setTimeout(resolve, 10)); });
    fireEvent.click(screen.getByText("运行能力与搜索"));
    await act(async () => { await new Promise(resolve => setTimeout(resolve, 10)); });
    expect(screen.getByRole("checkbox", { name: "流式接收" })).not.toBeChecked();
  });
});
