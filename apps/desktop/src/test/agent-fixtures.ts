import type { AgentEventSnapshot, AgentSnapshot } from "@meowlive/contracts";

export function agentEvent(overrides: Partial<AgentEventSnapshot> = {}): AgentEventSnapshot {
  return {
    event: { id: "event-1", source: "simulator", viewer: "小猫", kind: { type: "chat", text: "晚上好" } },
    status: "pending",
    speech_id: null,
    error: null,
    ...overrides,
  };
}

export function agentStatus(overrides: Partial<AgentSnapshot> = {}): AgentSnapshot {
  return {
    paused: true,
    phase: "paused",
    settings: { persona: "温柔的猫娘主播", topic: "轻松聊天", proactive_enabled: false, cooldown_ms: 30_000 },
    events: [],
    last_error: null,
    current_speech_id: null,
    llm_configured: false,
    bridge_connected: false,
    ...overrides,
  };
}
