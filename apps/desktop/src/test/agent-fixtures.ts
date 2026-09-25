import type { AgentEventSnapshot, AgentSnapshot, InteractionSettings } from "@meowlive/contracts";

export const interactionSettings: InteractionSettings = {
  chat_read_mode: "auto", welcome_enabled: true,
  busy_chat_count: 6, busy_enter_count: 3, busy_pending_count: 4,
  welcome_cooldown_ms: 30_000, welcome_viewer_cooldown_ms: 600_000,
};

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
    settings: { persona: "温柔的猫娘主播", system_prompt: "", topic: "轻松聊天", proactive_enabled: false, cooldown_ms: 30_000, interaction: { ...interactionSettings } },
    events: [],
    last_error: null,
    current_speech_id: null,
    llm_configured: false,
    bridge_connected: false,
    ...overrides,
  };
}
