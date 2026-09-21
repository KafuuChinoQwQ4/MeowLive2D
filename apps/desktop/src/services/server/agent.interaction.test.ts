import { expect, it, vi } from "vitest";
import { agentStatus } from "../../test/agent-fixtures";
import { jsonResponse } from "../../test/server-fixtures";
import { createAgentClient } from "./agent";

const superChat = { type: "super_chat", text: "第一次来看直播", amount_cny: 30, start_at_ms: 1_700_000_000_000, end_at_ms: 1_700_000_060_000 };

function snapshotWithKind(kind: unknown) {
  return { ...agentStatus(), events: [{ event: { id: "e", source: "bilibili", viewer: "小猫", kind }, status: "pending", speech_id: null, error: null }] };
}

it.each([superChat, { type: "room_enter" }])("接受新增事件 $type 并保留完整载荷", async (kind) => {
  const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(snapshotWithKind(kind)));
  expect((await createAgentClient({ fetcher }).getStatus()).events[0].event.kind).toEqual(kind);
});

it.each([superChat, { type: "room_enter" }])("拒绝 $type 混入礼物元数据", async (kind) => {
  const snapshot = snapshotWithKind(kind);
  const value = { ...snapshot, events: snapshot.events.map(item => ({ ...item, event: { ...item.event, gift_metadata: { price: 1000 } } })) };
  const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
  await expect(createAgentClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "invalid_response" });
});

it.each([
  { ...superChat, amount_cny: 0 }, { ...superChat, amount_cny: 1.5 },
  { ...superChat, amount_cny: 1_000_001 }, { ...superChat, text: " " },
  { ...superChat, text: "猫".repeat(501) }, { ...superChat, start_at_ms: -1 },
  { ...superChat, end_at_ms: superChat.start_at_ms }, { ...superChat, end_at_ms: Number.MAX_SAFE_INTEGER + 1 },
])("拒绝损坏的 SC 载荷 %#", async (kind) => {
  const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(snapshotWithKind(kind)));
  await expect(createAgentClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "invalid_response" });
});

it.each([
  { chat_read_mode: "random" }, { welcome_enabled: "true" }, { busy_chat_count: 0 },
  { busy_chat_count: 1001 }, { busy_enter_count: 0 }, { busy_enter_count: 1001 },
  { busy_pending_count: 0 }, { busy_pending_count: 513 }, { welcome_cooldown_ms: 999 },
  { welcome_cooldown_ms: 3_600_001 }, { welcome_viewer_cooldown_ms: 999 },
  { welcome_viewer_cooldown_ms: 86_400_001 },
])("拒绝无效互动配置 %#", async (patch) => {
  const snapshot = agentStatus();
  const value = { ...snapshot, settings: { ...snapshot.settings, interaction: { ...snapshot.settings.interaction, ...patch } } };
  const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(value));
  await expect(createAgentClient({ fetcher }).getStatus()).rejects.toMatchObject({ code: "invalid_response" });
});
