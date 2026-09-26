import { interactionSettings } from "../../test/agent-fixtures";
import { describe, expect, it, vi } from "vitest";
import type { AgentSnapshot, EventBatchRequest } from "@meowlive/contracts";
import { jsonResponse } from "../../test/server-fixtures";
import { createAgentClient } from "./agent";

function agentSnapshot(overrides: Partial<AgentSnapshot> = {}): AgentSnapshot {
  return {
    paused: true,
    phase: "paused",
    settings: {
      persona: "温柔的猫娘主播",
      system_prompt: "",
      topic: "轻松聊天",
      proactive_enabled: false,
      cooldown_ms: 30_000,
      interaction: { ...interactionSettings },
    },
    events: [],
    last_error: null,
    current_speech_id: null,
    llm_configured: false,
    bridge_connected: false,
    ...overrides,
  };
}

describe("Agent HTTP 请求", () => {
  it("读取和选择本地人物卡配置，拒绝无效配置列表", async () => {
    const profiles = { profiles: [
      { id: "card-1", name: "魔女", persona: "月见镇魔女" },
      { id: "card-2", name: "侦探", persona: "冷静的侦探" },
    ], selected_profile_id: "card-1", storage_available: true };
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse(profiles))
      .mockResolvedValueOnce(jsonResponse({ ...profiles, selected_profile_id: "card-2" }))
      .mockResolvedValueOnce(jsonResponse({ ...profiles, selected_profile_id: "missing" }));
    const client = createAgentClient({ fetcher });

    await expect(client.getPersonaProfiles()).resolves.toEqual(profiles);
    await expect(client.selectPersonaProfile({ id: "card-2" })).resolves.toMatchObject({ selected_profile_id: "card-2" });
    expect(fetcher).toHaveBeenNthCalledWith(1, "http://127.0.0.1:19600/api/agent/personas", expect.objectContaining({ method: "GET" }));
    expect(fetcher).toHaveBeenNthCalledWith(2, "http://127.0.0.1:19600/api/agent/personas/select", expect.objectContaining({
      method: "POST", body: JSON.stringify({ id: "card-2" }),
    }));
    await expect(client.getPersonaProfiles()).rejects.toThrow("无效的人物卡配置");
  });

  it("以人物卡 ID 原子更新标题和内容", async () => {
    const profiles = { profiles: [{ id: "card-1", name: "旧名", persona: "旧设定" }], selected_profile_id: "card-1", storage_available: true };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ ...profiles, profiles: [{ id: "card-1", name: "新名", persona: "新设定" }] }));
    await expect(createAgentClient({ fetcher }).updatePersonaProfile({ id: "card-1", name: "新名", persona: "新设定" })).resolves.toMatchObject({ selected_profile_id: "card-1" });
    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:19600/api/agent/personas/update", expect.objectContaining({
      method: "POST", body: JSON.stringify({ id: "card-1", name: "新名", persona: "新设定" }),
    }));
  });

  it("读取 Agent 状态", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(agentSnapshot()));
    const client = createAgentClient({ baseUrl: "http://localhost:19700/", fetcher });

    await expect(client.getStatus()).resolves.toMatchObject({ paused: true, phase: "paused" });
    expect(fetcher).toHaveBeenCalledWith("http://localhost:19700/api/agent", expect.objectContaining({ method: "GET" }));
  });

  it("原样保存设置并接受暂停后的快照", async () => {
    const settings = { persona: "冷静的主持人", system_prompt: "直接读出弹幕", topic: "动作游戏", proactive_enabled: true, cooldown_ms: 45_000, interaction: { ...interactionSettings } };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(agentSnapshot({ settings })));

    await expect(createAgentClient({ fetcher }).saveSettings(settings)).resolves.toMatchObject({ paused: true, settings });
    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:19600/api/agent/settings", expect.objectContaining({
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(settings),
    }));
  });

  it.each([
    ["pause", "/api/agent/pause"],
    ["resume", "/api/agent/resume"],
  ] as const)("%s 调用对应控制路由", async (method, path) => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(agentSnapshot()));
    const client = createAgentClient({ fetcher });

    await client[method]();

    expect(fetcher).toHaveBeenCalledWith(`http://127.0.0.1:19600${path}`, expect.objectContaining({ method: "POST" }));
  });

  it("批量提交事件并读取 202 去重结果", async () => {
    const batch: EventBatchRequest = {
      events: [{ id: "event-1", source: "simulator", viewer: "小猫", kind: { type: "chat", text: "晚上好" } }],
    };
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse({ accepted: 0, duplicates: 1, persisted: 1, unscheduled: 0 }, 202));

    await expect(createAgentClient({ fetcher }).submitEvents(batch)).resolves.toEqual({ accepted: 0, duplicates: 1, persisted: 1, unscheduled: 0 });
    expect(fetcher).toHaveBeenCalledWith("http://127.0.0.1:19600/api/events", expect.objectContaining({
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(batch),
    }));
  });
});
